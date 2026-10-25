"""Collect a clearly labeled diagnostic build, never a production installer."""
import hashlib
import argparse
import json
import os
from pathlib import Path
import shutil
import sys

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("target")
parser.add_argument("--release-dir", type=Path)
args = parser.parse_args()
target = args.target
if target not in {"x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "aarch64-apple-darwin", "x86_64-apple-darwin"}:
    raise SystemExit("Unsupported target")
source = args.release_dir or root / "target" / target / "release"
output = root / "dist"
output.mkdir(exist_ok=True)
suffix = ".exe" if "windows" in target else ""
for binary in ["kyuutube-desktop-spike", "kyuutube-audio-spike"]:
    shutil.copy2(source / (binary + suffix), output)
shutil.copy2(root / "tools/fixtures/tone.wav", output)
shutil.copy2(root / "docs/FEASIBILITY_BUILD.md", output / "README.md")
shutil.copy2(root / "web/fonts/OFL.txt", output / "ROBOTO_FLEX_LICENSE.txt")
shutil.copy2(root / "docs/GOOGLE_ACCOUNT.md", output)
shutil.copy2(root / "docs/METROLIST_PARITY.md", output)
shutil.copy2(root / "tools/Measure-ProcessTree.ps1", output)
metadata = {"purpose": "publisher browser OAuth and experimental native Music playback; live service verification pending; no downloads",
            "target": target, "source_commit": os.environ.get("GITHUB_SHA", "local"),
            "runtime_validation": "See TEST_REPORT.md; a compile result is not a playback certification",
            "bundled_mpv": False,
            "auth_mode": "official-webview-session",
            "native_music_playback_verified": False,
            "official_player_test_preview": True,
            "publisher_oauth_configured": bool(os.environ.get("KYUUTUBE_GOOGLE_CLIENT_ID", "").strip()) and bool(os.environ.get("KYUUTUBE_GOOGLE_CLIENT_SECRET", "").strip())}
(output / "BUILD_INFO.json").write_text(json.dumps(metadata, indent=2) + "\n")
names = ["kyuutube-desktop-spike" + suffix, "kyuutube-audio-spike" + suffix, "tone.wav", "README.md", "ROBOTO_FLEX_LICENSE.txt", "BUILD_INFO.json", "Measure-ProcessTree.ps1", "GOOGLE_ACCOUNT.md", "METROLIST_PARITY.md"]
checksums = [hashlib.sha256((output / name).read_bytes()).hexdigest() + "  " + name for name in names]
(output / "SHA256SUMS.txt").write_text("\n".join(checksums) + "\n")
