"""Measure the native local-file engine against the configured virtual audio sink."""
import json
from pathlib import Path
import subprocess
import time

import psutil
from desktop_probe import measure

root = Path(__file__).resolve().parents[1]
process = subprocess.Popen([str(root / "target/release/kyuutube-audio-spike"),
                            str(root / "tools/fixtures/tone.wav"), "20"],
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
try:
    time.sleep(2)
    result = measure(psutil.Process(process.pid), 12)
    stdout, stderr = process.communicate(timeout=15)
    timeline = [json.loads(line) for line in stdout.splitlines()]
    assert process.returncode == 0, stderr
    assert timeline[-1]["position_seconds"] > 15, timeline
    result.update({"status": "passed", "test_kind": "Generated local WAV through libmpv and PulseAudio null sink; no physical sound or YouTube", "timeline_samples": timeline})
    (root / "reports/local-audio-probe.json").write_text(json.dumps(result, indent=2) + "\n")
finally:
    if process.poll() is None:
        process.terminate()
        process.wait(timeout=10)
