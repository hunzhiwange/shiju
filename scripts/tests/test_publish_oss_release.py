import contextlib
import hashlib
import http.server
import io
import json
from pathlib import Path
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch, Mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import prepare_oss_release as prepare
import publish_oss_release as publish


class Store:
    def __init__(self):
        self.objects = {}
        self.fail = None
        self.race = None
        self.unknown = False

    def check_versioning(self):
        pass

    def read(self, key):
        if self.fail == '403':
            raise ValueError('OSS read failed')
        return self.objects.get(key)

    def create(self, key, stream, cache):
        if self.fail == key:
            raise ValueError('upload failed')
        if key == publish.LATEST and self.race:
            self.objects[key] = self.race
        if key in self.objects:
            raise publish.RecoverablePut('conditional conflict')
        self.objects[key] = stream.read()
        if key == publish.LATEST and self.unknown:
            raise publish.RecoverablePut('unknown outcome')

    def verify(self, key, size, sha):
        if key not in self.objects:
            raise ValueError('missing object')
        publish.verify_stream(io.BytesIO(self.objects[key]), size, sha)


class PublishingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name).resolve() / 'v1.2.3'
        self.directory.mkdir()
        self.store = Store()
        assets = []
        for target, kind, name in prepare.asset_specs('1.2.3'):
            data = name.encode()
            (self.directory / name).write_bytes(data)
            assets.append(dict(name=name, size=len(data), digest='sha256:' + hashlib.sha256(data).hexdigest(),
                               browser_download_url=f'https://github.com/hunzhiwange/shiju/releases/download/v1.2.3/{name}'))
        self.manifest = prepare.build_manifest(dict(tag_name='v1.2.3', draft=False, prerelease=False,
            html_url='https://github.com/hunzhiwange/shiju/releases/tag/v1.2.3', published_at='2026-10-27T00:00:00Z', assets=assets), '1.2.3')
        self.save()

    def save(self):
        (self.directory / 'manifest.json').write_text(json.dumps(self.manifest))

    def run_publish(self, public=None):
        return publish.publish_release(self.directory, self.store, public or self.store.verify)

    def test_first_publish_and_idempotent_resume(self):
        self.run_publish()
        original = dict(self.store.objects)
        self.run_publish()
        self.assertEqual(original, self.store.objects)
        self.assertEqual(len(original), 8)
        self.assertEqual(json.loads(publish.receipt_path(self.directory).read_text())['stage'], 'complete')

    def test_invalid_local_inputs_never_write_latest(self):
        for field, value in [('schema_version', 2), ('assets', []), ('preview', True)]:
            with self.subTest(field=field):
                old = dict(self.manifest)
                self.manifest[field] = value
                self.save()
                with self.assertRaises(ValueError):
                    self.run_publish()
                self.assertNotIn(publish.LATEST, self.store.objects)
                self.manifest = old
        self.save()
        asset = self.directory / self.manifest['assets'][0]['name']
        asset.write_bytes(b'bad')
        with self.assertRaises(ValueError):
            self.run_publish()
        asset.unlink()
        with self.assertRaises(ValueError):
            self.run_publish()
        asset.symlink_to('/etc/hosts')
        with self.assertRaises(ValueError):
            self.run_publish()
        self.assertEqual(self.store.objects, {})

    def test_same_size_wrong_hash(self):
        asset = self.directory / self.manifest['assets'][0]['name']
        asset.write_bytes(b'x' * asset.stat().st_size)
        with self.assertRaises(ValueError):
            self.run_publish()
        self.assertEqual(self.store.objects, {})

    def test_remote_conflict_downgrade_and_unsupported_cas(self):
        for version, message in (('1.2.3', 'different manifest'), ('2.0.0', 'Downgrade'), ('1.0.0', 'no supported destination CAS')):
            remote = json.loads(json.dumps(self.manifest).replace('1.2.3', version))
            remote['published_at'] = 'different'
            data = json.dumps(remote).encode()
            self.store.objects[publish.LATEST] = data
            with self.assertRaisesRegex(ValueError, message):
                self.run_publish()
            self.assertEqual(self.store.objects, {publish.LATEST: data})

    def test_auth_failure(self):
        self.store.fail = '403'
        with self.assertRaises(ValueError):
            self.run_publish()
        self.assertEqual(self.store.objects, {})

    def test_partial_upload_resumes(self):
        key = publish.PREFIX + '/releases/v1.2.3/' + self.manifest['assets'][2]['name']
        self.store.fail = key
        with self.assertRaises(ValueError):
            self.run_publish()
        self.assertEqual(len(self.store.objects), 2)
        self.assertNotIn(publish.LATEST, self.store.objects)
        self.store.fail = None
        self.run_publish()
        self.assertEqual(len(self.store.objects), 8)

    def test_immutable_conflict(self):
        key = publish.PREFIX + '/releases/v1.2.3/' + self.manifest['assets'][0]['name']
        self.store.objects[key] = b'conflict'
        with self.assertRaises(ValueError):
            self.run_publish()
        self.assertEqual(self.store.objects, {key: b'conflict'})

    def test_public_failure_preserves_latest(self):
        def fail(*args):
            raise ValueError('public failure')
        with self.assertRaises(ValueError):
            self.run_publish(fail)
        self.assertNotIn(publish.LATEST, self.store.objects)

    def test_concurrent_first_creation_conflict(self):
        self.store.race = b'another publisher'
        with self.assertRaises(ValueError):
            self.run_publish()
        self.assertEqual(self.store.objects[publish.LATEST], self.store.race)

    def test_auth_error_on_latest_does_not_become_success(self):
        self.store.fail = publish.LATEST
        with self.assertRaises(ValueError):
            self.run_publish()
        self.assertNotIn(publish.LATEST, self.store.objects)
        self.assertEqual(json.loads(publish.receipt_path(self.directory).read_text())['stage'], 'publishing_latest')

    def test_latest_unknown_result_is_read_before_resume(self):
        self.store.unknown = True
        self.run_publish()
        self.assertEqual(self.store.objects[publish.LATEST], (self.directory / 'manifest.json').read_bytes())

    def test_latest_public_failure_receipt_and_resume(self):
        def verify(key, size, sha):
            if key == publish.LATEST:
                raise ValueError('public failure')
            self.store.verify(key, size, sha)
        with self.assertRaises(ValueError):
            self.run_publish(verify)
        original = self.store.objects[publish.LATEST]
        self.assertEqual(json.loads(publish.receipt_path(self.directory).read_text())['stage'], 'publishing_latest')
        self.run_publish()
        self.assertEqual(self.store.objects[publish.LATEST], original)

    def test_anonymous_http_stream_verification(self):
        data = b'installer' * 200000
        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                self.send_response(200)
                self.end_headers()
                self.wfile.write(data)
            def log_message(self, *args):
                pass
        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            url = f'http://127.0.0.1:{server.server_port}/asset'
            publish.verify_url(url, len(data), hashlib.sha256(data).hexdigest())
            with self.assertRaises(ValueError):
                publish.verify_url(url, len(data), '0' * 64)
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

    def test_preview_no_credentials_or_network(self):
        with patch.object(publish, 'load_credentials', side_effect=AssertionError), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(publish.main(['--dir', str(self.directory)]), 0)
        self.assertFalse(publish.receipt_path(self.directory).exists())

    def test_bucket_versioning_must_be_unconfigured(self):
        store = publish.OssStore.__new__(publish.OssStore)
        store.bucket = Mock()
        for state in ('Enabled', 'Suspended', '', 'unknown'):
            store.bucket.get_bucket_versioning.return_value.status = state
            with self.assertRaises(ValueError):
                store.check_versioning()
        store.bucket.get_bucket_versioning.return_value.status = None
        store.check_versioning()

    def test_only_explicit_no_such_key_is_absence(self):
        store = publish.OssStore.__new__(publish.OssStore)
        store.bucket = Mock()
        for status, code in ((403, 'AccessDenied'), (404, 'NoSuchBucket'), (500, 'InternalError')):
            error = RuntimeError('secret server payload')
            error.status, error.code = status, code
            store.bucket.get_object.side_effect = error
            with self.assertRaises(RuntimeError):
                store.read(publish.LATEST)
        error.status, error.code = 404, 'NoSuchKey'
        self.assertIsNone(store.read(publish.LATEST))

    def test_preview_only_and_unsafe_manifest_rejected(self):
        path = self.directory / 'manifest.json'
        path.rename(self.directory / 'manifest.preview.json')
        with self.assertRaises(ValueError):
            self.run_publish()
        path.symlink_to(self.directory / 'manifest.preview.json')
        with self.assertRaises(ValueError):
            self.run_publish()
        self.assertEqual(self.store.objects, {})

    def test_mapping_and_arbitrary_destinations_rejected(self):
        for field, value in (('url', 'https://example.com/evil'), ('name', '../evil'),
                             ('github_url', 'https://example.com/file'), ('target', 'unknown')):
            old = self.manifest['assets'][0][field]
            self.manifest['assets'][0][field] = value
            self.save()
            with self.assertRaises(ValueError):
                self.run_publish()
            self.assertEqual(self.store.objects, {})
            self.manifest['assets'][0][field] = old

    def test_credential_whitelist_and_environment_override(self):
        config = Path(self.temp.name) / '.config/shiju-oss-upload'
        config.mkdir(parents=True)
        (config / 'config.json').write_text(json.dumps(dict(region='cn-hangzhou',
            access_key_id='local-id', access_key_secret='local-secret', names={'mac': 'ignored'})))
        with patch.object(Path, 'home', return_value=Path(self.temp.name)), patch.dict(
                publish.os.environ, {'OSS_ACCESS_KEY_ID': 'env-id', 'OSS_ACCESS_KEY_SECRET': 'env-secret'}):
            self.assertEqual(publish.load_credentials(), ('cn-hangzhou', 'env-id', 'env-secret'))

    def test_credentials_errors_do_not_leak(self):
        output = io.StringIO()
        with patch.object(publish, 'load_credentials', side_effect=RuntimeError('SECRET_TOKEN')), contextlib.redirect_stderr(output), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(publish.main(['--dir', str(self.directory), '--publish']), 1)
        self.assertNotIn('SECRET_TOKEN', output.getvalue())


if __name__ == '__main__':
    unittest.main()
