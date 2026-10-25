"""Real WebKitGTK/Tauri UI checks in a virtual X11 session; not YouTube tests."""
import ctypes
import json
import os
from pathlib import Path
import statistics
import subprocess
import time
import urllib.request

import psutil

ROOT = Path(__file__).resolve().parents[1]
BASE = "http://127.0.0.1:4444"
OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def request(path, body=None, method=None):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(BASE + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    with OPENER.open(req, timeout=30) as response:
        value = json.load(response)["value"]
    if isinstance(value, dict) and "error" in value:
        raise RuntimeError(value)
    return value


def capture_window(prefix, path):
    """Capture the real X11 window; WebKit's screenshot endpoint can hang."""
    rect = request(prefix + "/window/rect")
    gdk = ctypes.CDLL("libgdk-3.so.0")
    pixbuf = ctypes.CDLL("libgdk_pixbuf-2.0.so.0")
    objects = ctypes.CDLL("libgobject-2.0.so.0")
    gdk.gdk_init.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
    gdk.gdk_init(None, None)
    gdk.gdk_get_default_root_window.restype = ctypes.c_void_p
    gdk.gdk_pixbuf_get_from_window.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int]
    gdk.gdk_pixbuf_get_from_window.restype = ctypes.c_void_p
    image = gdk.gdk_pixbuf_get_from_window(gdk.gdk_get_default_root_window(), rect['x'], rect['y'], rect['width'], rect['height'])
    if not image:
        raise RuntimeError("Native X11 window capture failed")
    pixbuf.gdk_pixbuf_savev.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]
    pixbuf.gdk_pixbuf_savev.restype = ctypes.c_int
    objects.g_object_unref.argtypes = [ctypes.c_void_p]
    try:
        if not pixbuf.gdk_pixbuf_savev(image, str(path).encode(), b'png', None, None, None):
            raise RuntimeError("Could not save native window capture")
    finally:
        objects.g_object_unref(image)


def native_window_titles():
    # Private webviews use a separate WebKit context and are not enumerated by
    # the launcher's WebDriver session. Inspect actual X11 top-level windows.
    x = ctypes.CDLL("libX11.so.6")
    x.XOpenDisplay.argtypes=[ctypes.c_char_p]; x.XOpenDisplay.restype=ctypes.c_void_p
    x.XDefaultRootWindow.argtypes=[ctypes.c_void_p]; x.XDefaultRootWindow.restype=ctypes.c_ulong
    x.XQueryTree.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.POINTER(ctypes.c_ulong),ctypes.POINTER(ctypes.c_ulong),ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)),ctypes.POINTER(ctypes.c_uint)]
    x.XFetchName.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.POINTER(ctypes.c_void_p)]
    x.XFree.argtypes=[ctypes.c_void_p]; x.XCloseDisplay.argtypes=[ctypes.c_void_p]
    display=x.XOpenDisplay(None)
    if not display: raise RuntimeError("X11 display unavailable")
    root=ctypes.c_ulong(); parent=ctypes.c_ulong(); children=ctypes.POINTER(ctypes.c_ulong)(); count=ctypes.c_uint()
    names=[]
    try:
        assert x.XQueryTree(display,x.XDefaultRootWindow(display),ctypes.byref(root),ctypes.byref(parent),ctypes.byref(children),ctypes.byref(count))
        for i in range(count.value):
            title=ctypes.c_void_p()
            if x.XFetchName(display,children[i],ctypes.byref(title)) and title.value:
                names.append(ctypes.string_at(title).decode('utf-8','replace')); x.XFree(title)
        if children: x.XFree(children)
    finally: x.XCloseDisplay(display)
    return names


def app_process():
    for p in psutil.process_iter(["name"]):
        try:
            if Path(p.exe()).name == "kyuutube-desktop-spike":
                return p
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
    raise RuntimeError("Desktop process not found")


def measure(process, seconds):
    samples = []
    previous = {}
    deadline = time.monotonic() + seconds
    previous_at = time.monotonic()
    while time.monotonic() < deadline:
        processes = [process] + process.children(recursive=True)
        rss = pss = 0
        cpu = 0.0
        per_process = []
        now = time.monotonic()
        for child in processes:
            try:
                memory = child.memory_full_info()
                usage = child.cpu_times()
                total = usage.user + usage.system
                if child.pid in previous:
                    cpu += max(0, total - previous[child.pid])
                previous[child.pid] = total
                rss += memory.rss
                pss += getattr(memory, "pss", 0)
                per_process.append({"pid": child.pid, "name": child.name(), "rss_mib": round(memory.rss / 2**20, 2),
                                    "pss_mib": round(getattr(memory, "pss", 0) / 2**20, 2)})
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                continue
        if samples:
            cpu_percent = 100 * cpu / max(0.001, now - previous_at)
        else:
            cpu_percent = None
        samples.append({"monotonic_seconds": round(now, 3), "rss_sum_mib": round(rss / 2**20, 2),
                        "pss_sum_mib": round(pss / 2**20, 2), "one_core_cpu_percent": cpu_percent,
                        "processes": per_process})
        previous_at = now
        time.sleep(0.5)
    percentages = [s["one_core_cpu_percent"] for s in samples if s["one_core_cpu_percent"] is not None]
    return {"duration_seconds": seconds, "rss_median_mib": statistics.median(s["rss_sum_mib"] for s in samples),
            "rss_max_mib": max(s["rss_sum_mib"] for s in samples),
            "pss_median_mib": statistics.median(s["pss_sum_mib"] for s in samples),
            "average_one_core_cpu_percent": round(statistics.mean(percentages), 2), "samples": samples}


def main():
    log = (ROOT / "reports/local-webdriver.log").open("w")
    driver = subprocess.Popen(["tauri-driver"], stdout=log, stderr=log)
    session = None
    report = {"test_kind": "local sample in Tauri/WebKitGTK under Xvfb; no physical audio or YouTube",
              "requested_graphics_policy": "never" if os.environ.get("KYUUTUBE_SPIKE_SOFTWARE_RENDER") else "default",
              "prelaunch_compositing_disabled": os.environ.get("WEBKIT_DISABLE_COMPOSITING_MODE") == "1",
              "screenshot_method": "native GDK capture of the actual X11 window",
              "checks": [], "stages": {}}
    try:
        for _ in range(100):
            if driver.poll() is not None:
                raise RuntimeError("WebDriver exited; inspect reports/local-webdriver.log")
            try:
                request("/status")
                break
            except Exception:
                time.sleep(0.1)
        result = request("/session", {"capabilities": {"alwaysMatch": {"tauri:options": {
            "application": str(ROOT / "target/release/kyuutube-desktop-spike")
        }}}})
        session = result["sessionId"]
        prefix = "/session/" + session

        def execute(script, args=None):
            return request(prefix + "/execute/sync", {"script": script, "args": args or []})

        def snapshot():
            return request(prefix + "/execute/async", {"script": "const done=arguments[arguments.length-1]; window.__TAURI__.core.invoke('snapshot').then(done,e=>done({error:String(e)}));", "args": []})

        def click(selector):
            found = request(prefix + "/element", {"using": "css selector", "value": selector})
            element = found["element-6066-11e4-a52e-4f735466cecf"]
            request(prefix + "/element/" + element + "/click", {})

        assert execute("return document.title") == "kYuutube"
        assert execute("return !!document.querySelector('#open-music') && !!document.querySelector('#open-browser') && !document.querySelector('#client-id')")
        report["checks"].append("Credential-free official-player launcher rendered in real Tauri webview")
        capture_window(prefix, ROOT / "reports/local-welcome.png")
        click("#open-music")
        time.sleep(1)
        status=execute("return document.querySelector('#official-status').textContent")
        assert status == '', status
        titles=native_window_titles()
        assert any('YouTube Music' in title for title in titles), titles
        report["checks"].append("Open Music created a separate remote player window; live site access remains blocked")
        report["status"] = "passed"
    except Exception as error:
        report["status"] = "failed"
        report["error"] = str(error)
        raise
    finally:
        (ROOT / "reports/local-desktop-probe.json").write_text(json.dumps(report, indent=2) + "\n")
        if session:
            try:
                request("/session/" + session, method="DELETE")
            except Exception:
                pass
        driver.terminate()
        try:
            driver.wait(timeout=10)
        except subprocess.TimeoutExpired:
            driver.kill()
            driver.wait()
        log.close()


if __name__ == "__main__":
    main()
