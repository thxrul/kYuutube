"""Verify and archive four CI diagnostics for an explicitly selected alpha."""
import argparse
import re
from pathlib import Path
import hashlib
import json
import tarfile
import zipfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('base', type=Path)
parser.add_argument('out', type=Path)
parser.add_argument('--source', default='508ecab573257b3c63eac9a6398397d039c35d37')
parser.add_argument('--tag', default='v0.0.1-alpha.1')
args = parser.parse_args()
if not re.fullmatch(r'[0-9a-f]{40}', args.source) or not re.fullmatch(r'v[0-9]+\.[0-9]+\.[0-9]+-alpha\.[0-9]+', args.tag):
    raise SystemExit('Expected a full source SHA and an alpha version tag')
base, out, source = args.base, args.out, args.source
out.mkdir(parents=True, exist_ok=True)
targets = {
    'x86_64-pc-windows-msvc': 'windows-x86_64',
    'x86_64-unknown-linux-gnu': 'linux-x86_64',
    'aarch64-apple-darwin': 'macos-arm64',
    'x86_64-apple-darwin': 'macos-x86_64',
}
archives = []
for target, label in targets.items():
    folder = base / ('feasibility-' + target)
    info = json.loads((folder / 'BUILD_INFO.json').read_text())
    if info['source_commit'] != source or info['target'] != target:
        raise ValueError('Unexpected artifact provenance')
    if info.get('auth_mode') == 'publisher-browser-oauth' and info.get('publisher_oauth_configured') is not True:
        raise ValueError('Publisher OAuth is missing. Configure the repository Actions secrets and rebuild before publishing this sign-in flow.')
    if info.get('native_music_playback_verified') is not True and not (info.get('auth_mode') == 'official-webview-session' and info.get('official_player_test_preview') is True):
        raise ValueError('Native YouTube Music playback has not been verified. Publication is on hold until a real authenticated track plays.')
    for line in (folder / 'SHA256SUMS.txt').read_text().splitlines():
        expected, name = line.split('  ', 1)
        if Path(name).name != name:
            raise ValueError('Unexpected artifact path')
        if hashlib.sha256((folder / name).read_bytes()).hexdigest() != expected:
            raise ValueError('Checksum mismatch: ' + name)
    name = 'kYuutube-' + args.tag + '-' + label
    if label.startswith('windows'):
        archive = out / (name + '.zip')
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
            for file in sorted(folder.iterdir()):
                bundle.write(file, name + '/' + file.name)
    else:
        archive = out / (name + '.tar.gz')
        with tarfile.open(archive, 'w:gz') as bundle:
            for file in sorted(folder.iterdir()):
                entry = bundle.gettarinfo(str(file), arcname=name + '/' + file.name)
                entry.mode = 0o755 if file.name.startswith('kyuutube-') else 0o644
                with file.open('rb') as stream:
                    bundle.addfile(entry, stream)
    archives.append(archive)
(out / 'SHA256SUMS.txt').write_text(''.join(
    hashlib.sha256(file.read_bytes()).hexdigest() + '  ' + file.name + '\n'
    for file in archives
))
