"""Production-style direct launch, without WebDriver/automation instrumentation."""
import json
import os
from pathlib import Path
import subprocess
import time

import psutil
from desktop_probe import measure

root = Path(__file__).resolve().parents[1]
environment = os.environ.copy()
environment.pop("TAURI_AUTOMATION", None)
log = (root / "reports/local-direct-window.log").open("w")
process = subprocess.Popen([str(root / "target/release/kyuutube-desktop-spike")],
                           env=environment, stdout=log, stderr=log)
try:
    time.sleep(8)
    assert process.poll() is None, "Direct-launch window exited during startup"
    result = measure(psutil.Process(process.pid), 15)
    result.update({"test_kind": "Direct release binary under Xvfb; no WebDriver, no playback", "status": "measured"})
    (root / "reports/local-direct-window.json").write_text(json.dumps(result, indent=2) + "\n")
finally:
    if process.poll() is None:
        process.terminate()
        process.wait(timeout=10)
    log.close()
