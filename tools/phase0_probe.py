"""Bounded, anonymous feasibility checks. No login, cookie import or media download.

Run with the Python interpreter from the research virtual environment. Results
measure these short Python probes, never desktop playback or Tauri overhead.
"""

import argparse
import json
import os
from pathlib import Path
import platform
import ssl
import subprocess
import sys
import time


def worker(name):
    if name == "search":
        import requests
        from ytmusicapi import YTMusic

        session = requests.Session()
        # Use the managed machine's CA trust, retaining certificate validation.
        session.verify = os.environ.get("REQUESTS_CA_BUNDLE") or os.environ.get(
            "SSL_CERT_FILE", ssl.get_default_verify_paths().cafile
        ) or True
        api = YTMusic(requests_session=session)
        results = api.search("YouTube Audio Library", filter="songs", limit=5)
        valid = [r for r in results if r.get("videoId")]
        print(json.dumps({"status": "passed" if valid else "failed", "result_count": len(valid)}))
        return 0 if valid else 1
    if name == "stream_resolution":
        import yt_dlp

        class Logger:
            def __init__(self):
                self.warnings = 0

            def debug(self, *_):
                pass

            def warning(self, *_):
                self.warnings += 1

            def error(self, *_):
                pass

        logger = Logger()
        # Metadata/manifest inspection only. Never persist URLs, cookies, tokens,
        # media, descriptions, or provider responses in the report.
        with yt_dlp.YoutubeDL({
            "quiet": True, "logger": logger, "skip_download": True,
            "socket_timeout": 15, "retries": 0, "extractor_retries": 0,
            "noplaylist": True, "cachedir": False,
        }) as api:
            info = api.extract_info("https://www.youtube.com/watch?v=jNQXAC9IVRw", download=False)
        formats = info.get("formats", [])
        playable = [f for f in formats if f.get("acodec") not in (None, "none") and f.get("url")]
        print(json.dumps({"status": "passed" if playable else "failed",
                          "formats_with_audio": len(playable), "warning_count": logger.warnings,
                          "playback_tested": False}))
        return 0 if playable else 1
    raise ValueError("Unknown probe")


def measure(name, timeout):
    import psutil

    started = time.monotonic()
    process = subprocess.Popen([sys.executable, __file__, "--worker", name],
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    root = psutil.Process(process.pid)
    peak_rss = 0
    cpu_by_pid = {}
    timed_out = False
    while process.poll() is None:
        try:
            tree = [root] + root.children(recursive=True)
        except psutil.NoSuchProcess:
            tree = []
        rss = 0
        for child in tree:
            try:
                rss += child.memory_info().rss
                cpu = child.cpu_times()
                cpu_by_pid[child.pid] = cpu.user + cpu.system
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                continue
        peak_rss = max(peak_rss, rss)
        if time.monotonic() - started > timeout:
            timed_out = True
            for child in reversed(tree):
                try:
                    child.kill()
                except psutil.NoSuchProcess:
                    pass
            process.kill()
            break
        time.sleep(0.05)
    stdout, _ = process.communicate(timeout=10)
    elapsed = time.monotonic() - started
    try:
        result = json.loads(stdout)
    except (ValueError, TypeError):
        result = {"status": "failed", "reason": "No valid result; provider response omitted"}
    return {"probe": name, "exit_code": process.returncode, "timed_out": timed_out,
            "wall_seconds": round(elapsed, 3), "sampled_peak_rss_mib": round(peak_rss / 2**20, 2),
            "sampled_cpu_seconds": round(sum(cpu_by_pid.values()), 3),
            "sampled_average_one_core_cpu_percent": round(100 * sum(cpu_by_pid.values()) / elapsed, 2),
            "result": result}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", choices=["search", "stream_resolution"])
    parser.add_argument("--output", type=Path, default=Path("reports/phase0-probes.json"))
    parser.add_argument("--timeout", type=int, default=90)
    args = parser.parse_args()
    if args.worker:
        try:
            return worker(args.worker)
        except Exception as error:
            # Error types are safe to record; messages can contain provider data.
            print(json.dumps({"status": "failed", "error_type": type(error).__name__}))
            return 1
    from importlib.metadata import version

    results = {
        "environment": {"os": platform.system(), "architecture": platform.machine(),
                        "python": platform.python_version(),
                        "packages": {n: version(n) for n in ["ytmusicapi", "yt-dlp", "psutil"]}},
        "measurement": "50ms RSS sampling, summed process-tree CPU time; short anonymous Python probes only",
        "authenticated_login": "not_run_missing_client_and_interactive_authorization",
        "audio_playback": "not_run",
        "probes": [measure(name, args.timeout) for name in ["search", "stream_resolution"]],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results, indent=2))
    return 0 if all(p["exit_code"] == 0 and p["result"]["status"] == "passed" for p in results["probes"]) else 1


if __name__ == "__main__":
    sys.exit(main())
