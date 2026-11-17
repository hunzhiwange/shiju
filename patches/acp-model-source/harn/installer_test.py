"""Pinned archive layout: unused official aliases may be skipped, other links are refused."""
import hashlib
import io
import json
from pathlib import Path
import runpy
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch


class ArchiveLayout(unittest.TestCase):
    def install(self, link_name, link_target):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            assets, runtime = root / 'assets', root / 'runtime'
            (assets / 'harn').mkdir(parents=True)
            pipeline, launcher, binary = b'pipeline fixture', b'launcher fixture', b'native fixture'
            (assets / 'harn/agent.harn').write_bytes(pipeline)
            (assets / 'harn/launcher.py').write_bytes(launcher)
            archive = io.BytesIO()
            with tarfile.open(fileobj=archive, mode='w:gz') as package:
                for name, data in [('harn', binary), ('LICENSE-MIT', b'license')]:
                    member = tarfile.TarInfo(name)
                    member.size = len(data)
                    package.addfile(member, io.BytesIO(data))
                member = tarfile.TarInfo(link_name)
                member.type = tarfile.SYMTYPE
                member.linkname = link_target
                package.addfile(member)
            data = archive.getvalue()
            digest = lambda value: hashlib.sha256(value).hexdigest()
            (assets / 'contracts.json').write_text(json.dumps({'harn': {
                'artifact_url': 'https://example.invalid/pinned-archive',
                'archive_sha256': digest(data), 'files': {
                    'harn': digest(launcher), 'harn-native': digest(binary),
                    'agent.harn': digest(pipeline)}}}))
            installer = Path(__file__).resolve().parents[3] / 'crates/libs/acp-agent-registry/src/managed_runtime/install.py'
            with patch.object(sys, 'argv', [str(installer), str(assets), str(runtime), 'harn', '']), \
                    patch('urllib.request.urlopen', return_value=io.BytesIO(data)), \
                    patch('platform.system', return_value='Darwin'), \
                    patch('platform.machine', return_value='arm64'):
                with self.assertRaises(SystemExit) as result:
                    runpy.run_path(str(installer), run_name='__main__')
                self.assertEqual(result.exception.code, 0)
            self.assertEqual((runtime / 'harn-native').read_bytes(), binary)
            self.assertTrue((runtime / 'LICENSE-MIT').is_file())
            self.assertFalse((runtime / link_name).exists())

    def test_official_native_alias_is_not_extracted(self):
        self.install('harn-dap', 'harn')

    def test_unexpected_archive_link_is_rejected(self):
        for name in ['unexpected', 'harn-dap']:
            with self.subTest(name=name):
                with self.assertRaisesRegex(RuntimeError, 'Unsafe Harn archive entry'):
                    self.install(name, '../outside')


if __name__ == '__main__':
    unittest.main()
