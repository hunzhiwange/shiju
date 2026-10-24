"""Exercise publishing failures and retries without registry writes or builds."""

import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import urllib.error

SCRIPTS = Path(__file__).resolve().parents[1]


def load(name):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / (name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


npm = load('publish_npm_release')
oidc = load('check_npm_publish_access')


class PackagePublishingTests(unittest.TestCase):
    def fixtures(self, root):
        artifacts = {}
        for name in oidc.PACKAGES:
            path = root / (name + '-npm-package.tar.gz')
            data = json.dumps({'name': '@shiju-ai/' + name, 'version': '0.5.6'}).encode()
            with tarfile.open(path, 'w:gz') as archive:
                member = tarfile.TarInfo('package/package.json')
                member.size = len(data)
                archive.addfile(member, io.BytesIO(data))
            artifacts[path.name] = {'checksums': {'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}}
        (root / 'dist-manifest.json').write_text(json.dumps({
            'announcement_tag': 'v0.5.6', 'artifacts': artifacts,
        }))

    def test_matching_versions_skip_and_missing_versions_publish(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.fixtures(root)

            def registry(url, timeout):
                if 'shiju-acp/' in url:
                    raise urllib.error.HTTPError(url, 404, 'missing', {}, None)
                name = url.split('%2F')[1].split('/')[0]
                digest = hashlib.sha1((root / (name + '-npm-package.tar.gz')).read_bytes()).hexdigest()
                return io.BytesIO(json.dumps({'dist': {'shasum': digest}}).encode())

            with patch.object(npm.urllib.request, 'urlopen', side_effect=registry), patch.object(npm.subprocess, 'run') as publish:
                npm.publish(root, 'v0.5.6')
                self.assertEqual(publish.call_count, 1)
                self.assertIn('shiju-acp-npm-package.tar.gz', publish.call_args.args[0][2])

    def test_invalid_artifact_blocks_all_publishes(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.fixtures(root)
            (root / 'shiju-webview-npm-package.tar.gz').write_bytes(b'corrupt')
            with patch.object(npm.subprocess, 'run') as publish:
                with self.assertRaisesRegex(SystemExit, 'checksum mismatch'):
                    npm.publish(root, 'v0.5.6')
                publish.assert_not_called()

    def test_different_published_payload_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.fixtures(root)
            with patch.object(npm.urllib.request, 'urlopen', return_value=io.BytesIO(b'{"dist":{"shasum":"different"}}')), patch.object(npm.subprocess, 'run') as publish:
                with self.assertRaisesRegex(SystemExit, 'differs from release'):
                    npm.publish(root, 'v0.5.6')
                publish.assert_not_called()

    def test_oidc_exchanges_all_packages_without_logging_tokens(self):
        requests = []

        def registry(request, timeout):
            requests.append(request)
            data = {'value': 'identity-secret'} if len(requests) == 1 else {'token': 'npm-secret'}
            return io.BytesIO(json.dumps(data).encode())

        output = io.StringIO()
        with patch.dict(os.environ, ACTIONS_ID_TOKEN_REQUEST_URL='https://example.test/?request=1', ACTIONS_ID_TOKEN_REQUEST_TOKEN='github-secret'), patch.object(oidc.urllib.request, 'urlopen', side_effect=registry), contextlib.redirect_stdout(output):
            oidc.check_access()
        self.assertEqual(len(requests), 5)
        self.assertIn('audience=npm%3Aregistry.npmjs.org', requests[0].full_url)
        self.assertIn('%40shiju-ai%2Fshiju', requests[1].full_url)
        self.assertNotIn('secret', output.getvalue())

    def test_homebrew_repeat_is_a_noop(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            bare, tap, source = root / 'remote.git', root / 'tap', root / 'formula'
            source.mkdir()
            subprocess.run(['git', 'init', '--bare', str(bare)], check=True, capture_output=True)
            subprocess.run(['git', 'clone', str(bare), str(tap)], check=True, capture_output=True)
            (tap / 'Formula').mkdir()
            (tap / 'Formula/.keep').touch()
            for args in (['add', '.'], ['-c', 'user.name=Test', '-c', 'user.email=test@example.test', 'commit', '-m', 'initial']):
                subprocess.run(['git', '-C', str(tap), *args], check=True, capture_output=True)
            for name in oidc.PACKAGES:
                (source / (name + '.rb')).write_text('# existing release formula\n')
            command = ['bash', str(SCRIPTS / 'publish_homebrew.sh'), str(source), str(tap), 'v0.5.6']
            subprocess.run(command, check=True, capture_output=True)
            before = subprocess.check_output(['git', '-C', str(tap), 'rev-parse', 'HEAD'])
            result = subprocess.run(command, check=True, capture_output=True, text=True)
            after = subprocess.check_output(['git', '-C', str(tap), 'rev-parse', 'HEAD'])
            self.assertEqual(before, after)
            self.assertIn('already match', result.stdout)


if __name__ == '__main__':
    unittest.main()
