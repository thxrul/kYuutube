"""Exercise the publisher gate through the real release-packaging command."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
TARGETS = ['x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu',
           'aarch64-apple-darwin', 'x86_64-apple-darwin']
SOURCE = 'a' * 40


class ReleaseProvisioning(unittest.TestCase):
    def package(self, directory, provisioned, music_verified=True):
        base = directory / 'artifacts'
        for target in TARGETS:
            folder = base / ('feasibility-' + target)
            folder.mkdir(parents=True)
            metadata = folder / 'BUILD_INFO.json'
            metadata.write_text(json.dumps({'target': target, 'source_commit': SOURCE,
                'auth_mode': 'publisher-browser-oauth',
                'publisher_oauth_configured': provisioned, 'native_music_playback_verified': music_verified}))
            # Fictitious CI payload; no Google registration or binary is needed
            # to check whether the publication gate admits this manifest.
            digest = hashlib.sha256(metadata.read_bytes()).hexdigest()
            (folder / 'SHA256SUMS.txt').write_text(digest + '  BUILD_INFO.json\n')
        output = directory / 'release'
        result = subprocess.run([sys.executable, str(ROOT / 'tools/package_alpha.py'),
            str(base), str(output), '--source', SOURCE, '--tag', 'v0.0.1-alpha.3'],
            capture_output=True, text=True)
        return result, output

    def test_missing_publisher_registration_blocks_publication(self):
        with tempfile.TemporaryDirectory() as temp:
            result, output = self.package(Path(temp), False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Publisher OAuth is missing', result.stderr)
            self.assertEqual(list(output.iterdir()), [])

    def test_google_registration_alone_cannot_publish_unverified_native_music(self):
        with tempfile.TemporaryDirectory() as temp:
            result, output = self.package(Path(temp), True, False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Native YouTube Music playback has not been verified', result.stderr)
            self.assertEqual(list(output.iterdir()), [])

    def test_configured_manifest_can_be_archived_with_verified_checksums(self):
        with tempfile.TemporaryDirectory() as temp:
            result, output = self.package(Path(temp), True)
            self.assertEqual(result.returncode, 0, result.stderr)
            lines = (output / 'SHA256SUMS.txt').read_text().splitlines()
            self.assertEqual(len(lines), 4)
            for line in lines:
                expected, name = line.split('  ', 1)
                self.assertEqual(hashlib.sha256((output / name).read_bytes()).hexdigest(), expected)


if __name__ == '__main__':
    unittest.main()
