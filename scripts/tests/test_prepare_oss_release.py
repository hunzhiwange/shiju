import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import unittest
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location('prepare', Path(__file__).parents[1] / 'prepare_oss_release.py')
prepare = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(prepare)
BODY = b'neutral fixture installer\x00' * 8192


def fixture():
    return dict(tag_name='v0.5.10', draft=False, prerelease=False,
                published_at='2026-10-26T00:00:00Z',
                html_url='https://github.com/hunzhiwange/shiju/releases/tag/v0.5.10',
                assets=[dict(name=name, size=len(BODY), digest='sha256:' + hashlib.sha256(BODY).hexdigest(),
                             browser_download_url=f'https://github.com/hunzhiwange/shiju/releases/download/v0.5.10/{name}')
                        for _, _, name in prepare.asset_specs('0.5.10')])


def prepared_contract():
    """Return actual generated installers and manifest, using local HTTP only."""
    import contextlib
    import io

    DownloadTests.setUpClass()
    try:
        with tempfile.TemporaryDirectory() as root:
            original_open = prepare.urllib.request.urlopen
            def local_download(url, **kwargs):
                return original_open(f'http://127.0.0.1:{DownloadTests.server.server_port}/ok', **kwargs)
            with patch.object(prepare, 'fetch_release', return_value=fixture()), patch.object(
                    prepare.urllib.request, 'urlopen', side_effect=local_download), contextlib.redirect_stdout(io.StringIO()):
                path = prepare.prepare_release('0.5.10', str(Path(root).resolve()))
            manifest = json.loads(path.read_bytes())
            assert all(prepare.verify_file(path.parent / asset["name"], asset)
                       for asset in manifest["assets"])
            return dict(manifest=manifest, github=fixture(), body=list(BODY))
    finally:
        DownloadTests.tearDownClass()


class ReleaseTests(unittest.TestCase):
    def test_versions(self):
        self.assertEqual(prepare.normalize_version('v0.5.10'), '0.5.10')
        for version in ['../1.2.3', '1.2', '1.2.3-beta', '1.2.3/4', '-1.2.3', '01.2.3']:
            with self.subTest(version=version), self.assertRaises(ValueError):
                prepare.normalize_version(version)

    def test_exact_assets_and_determinism(self):
        data = fixture()
        data['assets'].append(dict(name='unrelated.zip'))
        first = prepare.build_manifest(data, '0.5.10')
        data['assets'].reverse()
        self.assertEqual(first, prepare.build_manifest(data, '0.5.10'))
        self.assertEqual(len(first['assets']), 8)
        self.assertTrue(all('/agents/shiju-agent/releases/v0.5.10/' in a['url'] for a in first['assets']))

    def test_linux_deb_packages_are_required_for_both_architectures(self):
        manifest = prepare.build_manifest(fixture(), '0.5.10')
        debs = {a['target']: a['name'] for a in manifest['assets'] if a['kind'] == 'deb'}
        self.assertEqual(debs, {
            'aarch64-unknown-linux-gnu': 'shiju_0.5.10_arm64.deb',
            'x86_64-unknown-linux-gnu': 'shiju_0.5.10_amd64.deb',
        })
        data = fixture()
        data['assets'] = [a for a in data['assets'] if not a['name'].endswith('.deb')]
        with self.assertRaisesRegex(ValueError, 'deb'):
            prepare.build_manifest(data, '0.5.10')

    def test_reject_invalid_metadata(self):
        for mutation in [lambda d: d['assets'].pop(), lambda d: d['assets'].append(d['assets'][0]),
                         lambda d: d.update(tag_name='v1.2.3'), lambda d: d.update(draft=True),
                         lambda d: d.update(prerelease=True), lambda d: d['assets'][0].update(name='../bad.rpm'),
                         lambda d: d['assets'][0].pop('digest'), lambda d: d['assets'][0].update(size=True),
                         lambda d: d['assets'][0].update(digest='sha256:bad'),
                         lambda d: d['assets'][0].update(browser_download_url='https://evil.invalid/a')]:
            data = fixture()
            mutation(data)
            with self.subTest(data=data), self.assertRaises(ValueError):
                prepare.build_manifest(data, '0.5.10')

    def test_preview_never_downloads_and_is_repeatable(self):
        with tempfile.TemporaryDirectory() as root, patch.object(prepare, 'fetch_release', return_value=fixture()), patch.object(prepare.urllib.request, 'urlopen', side_effect=AssertionError('download')):
            path = prepare.prepare_release('0.5.10', root, True)
            original = path.read_bytes()
            self.assertEqual(path.name, 'manifest.preview.json')
            self.assertFalse(path.with_name('manifest.json').exists())
            self.assertEqual(prepare.prepare_release('0.5.10', root, True).read_bytes(), original)

    def test_api_redirect_rejected(self):
        handler = prepare.NoRedirect()
        with self.assertRaises(ValueError):
            handler.redirect_request(None, None, 302, 'redirect', {}, 'https://elsewhere.invalid')


class MetadataTests(unittest.TestCase):
    def test_environment_credentials_take_priority_over_cli(self):
        for env, expected in [({'GITHUB_TOKEN': 'first', 'GH_TOKEN': 'second'}, 'first'),
                              ({'GH_TOKEN': 'second'}, 'second')]:
            with self.subTest(env=env), patch.dict('os.environ', env, clear=True), patch.object(
                    prepare.subprocess, 'run') as run:
                self.assertEqual(prepare.github_token(), expected)
                run.assert_not_called()

    def test_cli_credentials_and_anonymous_fallback(self):
        for result, expected in [(Mock(returncode=0, stdout='cli-token\n'), 'cli-token'),
                                 (Mock(returncode=1, stdout=''), None)]:
            with self.subTest(expected=expected), patch.dict('os.environ', {}, clear=True), patch.object(
                    prepare.subprocess, 'run', return_value=result) as run:
                self.assertEqual(prepare.github_token(), expected)
                self.assertEqual(run.call_args.args[0], ['gh', 'auth', 'token', '--hostname', 'github.com'])
        with patch.dict('os.environ', {}, clear=True), patch.object(
                prepare.subprocess, 'run', side_effect=FileNotFoundError):
            self.assertIsNone(prepare.github_token())

    def test_metadata_uses_credentials_only_on_api_request(self):
        opener = Mock()
        opener.open.return_value = io.BytesIO(json.dumps(fixture()).encode())
        with patch.object(prepare, 'github_token', return_value='secret'), patch.object(
                prepare.urllib.request, 'build_opener', return_value=opener):
            self.assertEqual(prepare.fetch_release('0.5.10'), fixture())
        request = opener.open.call_args.args[0]
        self.assertEqual(request.full_url, 'https://api.github.com/repos/hunzhiwange/shiju/releases/tags/v0.5.10')
        self.assertEqual(request.get_header('Authorization'), 'Bearer secret')

    def test_http_failures_are_actionable_and_do_not_expose_response_secrets(self):
        for status, headers, expected in [(403, {'X-RateLimit-Remaining': '0'}, 'rate limit exhausted'),
                                          (401, {}, 'credentials rejected'),
                                          (404, {}, 'published tag v0.5.10'),
                                          (403, {}, 'access forbidden')]:
            body = io.BytesIO(b'sensitive response')
            error = prepare.urllib.error.HTTPError('https://api.github.com', status, 'secret', headers, body)
            opener = Mock()
            opener.open.side_effect = error
            with self.subTest(status=status, headers=headers), patch.object(
                    prepare, 'github_token', return_value=None), patch.object(
                    prepare.urllib.request, 'build_opener', return_value=opener), patch.object(
                    prepare.time, 'sleep') as sleep:
                with self.assertRaisesRegex(ValueError, expected) as caught:
                    prepare.fetch_release('0.5.10')
                self.assertNotIn('secret', str(caught.exception))
                self.assertNotIn('sensitive', str(caught.exception))
                opener.open.assert_called_once()
                sleep.assert_not_called()
                self.assertTrue(body.closed)

    def test_server_failure_retries_then_succeeds(self):
        error = prepare.urllib.error.HTTPError('https://api.github.com', 503, 'Unavailable', {}, io.BytesIO())
        opener = Mock()
        opener.open.side_effect = [error, io.BytesIO(json.dumps(fixture()).encode())]
        with patch.object(prepare, 'github_token', return_value=None), patch.object(
                prepare.urllib.request, 'build_opener', return_value=opener), patch.object(
                prepare.time, 'sleep') as sleep:
            self.assertEqual(prepare.fetch_release('0.5.10'), fixture())
            self.assertEqual(opener.open.call_count, 2)
            sleep.assert_called_once_with(1)


class DownloadTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.requests = []
        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass
            def do_GET(self):
                cls.requests.append((self.path, self.headers.get('Authorization')))
                count = sum(p == self.path for p, _ in cls.requests)
                if self.path == '/retry' and count == 1:
                    self.send_error(503)
                    return
                self.send_response(200)
                self.send_header('Content-Length', str(len(BODY)))
                self.end_headers()
                self.wfile.write(BODY[:10] if self.path == '/truncated' else BODY)
        cls.server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join()

    def asset(self, endpoint):
        asset = prepare.build_manifest(fixture(), '0.5.10')['assets'][0]
        asset['github_url'] = f'http://127.0.0.1:{self.server.server_port}/{endpoint}'
        return asset

    def test_stream_retry_reuse_and_no_token(self):
        with tempfile.TemporaryDirectory() as root, patch.dict('os.environ', {'GITHUB_TOKEN': 'secret'}):
            asset = self.asset('retry')
            prepare.download_asset(asset, Path(root))
            path = Path(root) / asset['name']
            self.assertEqual(path.read_bytes(), BODY)
            count = len(self.requests)
            prepare.download_asset(asset, Path(root))
            self.assertEqual(len(self.requests), count)
            self.assertTrue(all(auth is None for _, auth in self.requests))
            path.write_bytes(b'different')
            with self.assertRaises(ValueError):
                prepare.download_asset(asset, Path(root))
            self.assertEqual(path.read_bytes(), b'different')

    def test_truncated_and_digest_mismatch_leave_no_complete_file(self):
        for endpoint in ['truncated', 'digest']:
            with self.subTest(endpoint=endpoint), tempfile.TemporaryDirectory() as root:
                asset = self.asset(endpoint)
                if endpoint == 'digest':
                    asset['sha256'] = '0' * 64
                with self.assertRaises(ValueError):
                    prepare.download_asset(asset, Path(root))
                self.assertFalse((Path(root) / asset['name']).exists())
                self.assertFalse(list(Path(root).glob('*.part')))

    def test_stale_part_and_manifest_only_after_all_verified(self):
        with tempfile.TemporaryDirectory() as root, patch.object(prepare, 'fetch_release', return_value=fixture()):
            original_open = prepare.urllib.request.urlopen
            def local_download(url, **kwargs):
                return original_open(f'http://127.0.0.1:{self.server.server_port}/ok', **kwargs)
            directory = Path(root) / 'v0.5.10'
            directory.mkdir()
            (directory / (fixture()['assets'][0]['name'] + '.part')).write_bytes(b'broken')
            with patch.object(prepare.urllib.request, 'urlopen', side_effect=local_download):
                path = prepare.prepare_release('0.5.10', root)
            self.assertEqual(len(json.loads(path.read_text())['assets']), 8)
            (directory / fixture()['assets'][-1]['name']).write_bytes(b'bad')
            with patch.object(prepare.urllib.request, 'urlopen', side_effect=local_download), self.assertRaises(ValueError):
                prepare.prepare_release('0.5.10', root)
            self.assertFalse(path.exists())


if __name__ == '__main__':
    unittest.main()
