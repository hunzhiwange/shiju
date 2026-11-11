"""Verify downloadable Web bundles serve independently of the source checkout."""
import hashlib
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import textwrap
import json
import os
import signal
import urllib.error
import unittest
import urllib.request
import zipfile

SCRIPTS = Path(__file__).resolve().parents[1]


def load_packager():
    spec = importlib.util.spec_from_file_location('package_shiju_web', SCRIPTS / 'package_shiju_web.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PackageShiJuWebTests(unittest.TestCase):
    def fixture(self, root):
        source = root / 'bindings'
        source.mkdir()
        (source / 'shi_ju.js').write_text('export default async function init() {}')
        (source / 'shi_ju_bg.wasm').write_bytes(b'\0asm\x01\0\0\0')
        (source / 'snippets' / 'runtime').mkdir(parents=True)
        (source / 'snippets' / 'runtime' / 'ime.js').write_text('export const ime = true;')
        return source

    def test_archive_has_relative_bootstrap_runtime_and_checksum(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = load_packager().package(self.fixture(root), root / 'out', '0.6.0-rc.1')
            with zipfile.ZipFile(archive) as bundle:
                base = 'shiju-web-0.6.0-rc.1/'
                html = bundle.read(base + 'index.html').decode()
                self.assertIn("import init from './shi_ju.js'", html)
                self.assertNotIn('data-trunk', html)
                self.assertNotIn('init().then', html)
                self.assertIn(base + 'snippets/runtime/ime.js', bundle.namelist())
                self.assertIn(base + 'serve.mjs', bundle.namelist())
                manifest = json.loads(bundle.read(base + 'package.json'))
                self.assertEqual(manifest['version'], '0.6.0-rc.1')
                self.assertTrue(manifest['private'])
                self.assertIn(base + 'README.md', bundle.namelist())
                self.assertFalse(any(name.endswith('.py') for name in bundle.namelist()))
                self.assertTrue(all(name.startswith(base) for name in bundle.namelist()))
            checksum = archive.with_suffix('.zip.sha256').read_text().split()
            self.assertEqual(checksum, [hashlib.sha256(archive.read_bytes()).hexdigest(), archive.name])

    def test_extracted_package_serves_without_repository(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = load_packager().package(self.fixture(root), root / 'out', '0.6.0')
            with zipfile.ZipFile(archive) as bundle:
                bundle.extractall(root / 'relocated')
            extracted = root / 'relocated' / 'shiju-web-0.6.0'
            server = subprocess.Popen(
                ['npm', 'run', 'start', '--', '--port', '0'],
                cwd=extracted, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                start_new_session=True,
            )
            try:
                url = ''
                for line in server.stdout:
                    if line.startswith('ShiJu Web: '):
                        url = line.strip().split()[-1]
                        break
                self.assertTrue(url.startswith('http://127.0.0.1:'))
                for filename, content_type in [('index.html', 'text/html'), ('shi_ju.js', 'text/javascript'), ('shi_ju_bg.wasm', 'application/wasm')]:
                    with urllib.request.urlopen(url + filename, timeout=5) as response:
                        self.assertEqual(response.status, 200)
                        self.assertEqual(response.headers.get_content_type(), content_type)
                        self.assertGreater(len(response.read()), 0)
                with urllib.request.urlopen(urllib.request.Request(url + 'shi_ju_bg.wasm', method='HEAD'), timeout=5) as response:
                    self.assertEqual(response.read(), b'')
                    self.assertGreater(int(response.headers['Content-Length']), 0)
                (root / 'outside.txt').write_text('outside package')
                (extracted / 'outside.txt').symlink_to(root / 'outside.txt')
                with self.assertRaises(urllib.error.HTTPError) as blocked:
                    urllib.request.urlopen(url + 'outside.txt', timeout=5)
                self.assertEqual(blocked.exception.code, 403)
            finally:
                if server.poll() is None:
                    os.killpg(server.pid, signal.SIGTERM)
                server.communicate(timeout=5)

    def test_missing_wasm_rejects_package(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = self.fixture(root)
            (source / 'shi_ju_bg.wasm').unlink()
            with self.assertRaisesRegex(ValueError, 'shi_ju_bg.wasm'):
                load_packager().package(source, root / 'out', '0.6.0')
            self.assertFalse((root / 'out' / 'shiju-web-0.6.0.zip').exists())

    def test_recovery_validates_web_zip_before_upload(self):
        workflow = (SCRIPTS.parent / '.github/workflows/release-recover.yml').read_text()
        marker = '          import hashlib, json, os, pathlib\n'
        block = marker + workflow.split(marker, 1)[1].split('\n          PY', 1)[0]
        code = textwrap.dedent(block)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            artifacts = root / 'artifacts'
            artifacts.mkdir()
            (artifacts / 'native.tar.gz').write_bytes(b'native payload')
            digest = hashlib.sha256(b'native payload').hexdigest()
            (artifacts / 'native.sha256').write_text(digest + '  native.tar.gz\n')
            (artifacts / 'dist-manifest.json').write_text(json.dumps({
                'announcement_tag': 'v0.6.0', 'announcement_is_prerelease': False,
                'announcement_github_body': 'Release notes',
                'artifacts': {'native.tar.gz': {'checksum': 'native.sha256'}},
            }))
            archive = load_packager().package(self.fixture(root), artifacts, '0.6.0')
            env = dict(os.environ, RUNNER_TEMP=str(root), RELEASE_TAG='v0.6.0')
            # Run the actual recovery verifier without GitHub writes.
            import sys
            valid = subprocess.run([sys.executable, '-c', code], cwd=root, env=env, capture_output=True, text=True)
            self.assertEqual(valid.returncode, 0, valid.stderr)
            archive.write_bytes(b'tampered')
            corrupt = subprocess.run([sys.executable, '-c', code], cwd=root, env=env, capture_output=True, text=True)
            self.assertNotEqual(corrupt.returncode, 0)
            self.assertIn('Web archive checksum mismatch', corrupt.stderr)
            archive.unlink()
            missing = subprocess.run([sys.executable, '-c', code], cwd=root, env=env, capture_output=True, text=True)
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn('Expected one complete Web archive', missing.stderr)

    def test_unsafe_version_and_symlinks_reject_package(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = self.fixture(root)
            packager = load_packager()
            with self.assertRaisesRegex(ValueError, 'version'):
                packager.package(source, root / 'out', '../outside')
            (source / 'snippets' / 'outside.js').symlink_to(SCRIPTS / 'serve_shiju_wasm.py')
            with self.assertRaisesRegex(ValueError, 'symlink'):
                packager.package(source, root / 'out', '0.6.0')


if __name__ == '__main__':
    unittest.main()
