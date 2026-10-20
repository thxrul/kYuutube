"""Real WebKitGTK/Tauri UI checks in a virtual X11 session; not YouTube tests."""
import base64
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

        assert execute("return document.title") == "kYuutube feasibility"
        report["checks"].append("Local UI loaded in the actual Tauri system webview")
        process = app_process()
        time.sleep(4)
        report["stages"]["idle"] = measure(process, 10)
        click("#play")
        # IPC completion starts asynchronous file loading. Wait for actual
        # timeline progress rather than assuming a two-second startup delay.
        deadline = time.monotonic() + 15
        start = snapshot()
        while (start.get("position_seconds") or 0) <= 0.2 and time.monotonic() < deadline:
            time.sleep(0.25)
            start = snapshot()
        assert (start.get("position_seconds") or 0) > 0.2, start
        report["stages"]["local_playing"] = measure(process, 15)
        later = snapshot()
        assert later["position_seconds"] > start["position_seconds"] + 5, (start, later)
        report["checks"].append("Play button started an advancing native local-file timeline")
        click("#pause")
        time.sleep(0.3)
        paused = snapshot()
        time.sleep(1)
        stable = snapshot()
        assert stable["paused"] and abs(stable["position_seconds"] - paused["position_seconds"]) < 0.2
        report["checks"].append("Pause button stopped the playback timeline")
        execute("const e=document.querySelector('#seek');e.value='5';e.dispatchEvent(new Event('change'))")
        time.sleep(0.5)
        assert abs(snapshot()["position_seconds"] - 5) < 0.5
        report["checks"].append("Seek control moved the paused timeline to five seconds")
        execute("const e=document.querySelector('#volume');e.value='25';e.dispatchEvent(new Event('change'))")
        report["checks"].append("Volume UI event submitted; hardware loudness not verified")
        for width, height in [(360, 480), (1920, 1080)]:
            request(prefix + "/window/rect", {"width": width, "height": height, "x": 0, "y": 0})
            time.sleep(0.3)
            assert execute("return document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1")
        report["checks"].append("Compact and wide window sizes had no horizontal overflow")
        request(prefix + "/window/rect", {"width": 960, "height": 680, "x": 0, "y": 0})
        screenshot = request(prefix + "/screenshot")
        (ROOT / "reports/local-spike.png").write_bytes(base64.b64decode(screenshot))
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
