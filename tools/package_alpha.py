"""Verify and archive the four CI diagnostics for the first alpha release."""
from pathlib import Path
import hashlib
import json
import sys
import tarfile
import zipfile

base, out = map(Path, sys.argv[1:])
out.mkdir(parents=True, exist_ok=True)
source = '508ecab573257b3c63eac9a6398397d039c35d37'
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
    for line in (folder / 'SHA256SUMS.txt').read_text().splitlines():
        expected, name = line.split('  ', 1)
        if Path(name).name != name:
            raise ValueError('Unexpected artifact path')
        if hashlib.sha256((folder / name).read_bytes()).hexdigest() != expected:
            raise ValueError('Checksum mismatch: ' + name)
    name = 'kYuutube-v0.0.1-alpha.1-' + label
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
