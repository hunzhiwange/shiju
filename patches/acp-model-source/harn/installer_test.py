"""Verify the pinned Harn source installer and its artifact receipt."""
import hashlib
import io
import json
import os
from pathlib import Path
import runpy
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch


SOURCE = 'harn-4914194ae9f56b0046210ca90ef6f927988e727e'
INSTALLER = (Path(__file__).resolve().parents[3] /
             'crates/libs/acp-agent-registry/src/managed_runtime/install.py')


def digest(data):
    return hashlib.sha256(data).hexdigest()


class SourceInstall(unittest.TestCase):
    def install(self, *, bad_patch=False, unsafe_member=False):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            assets, runtime, target = root / 'assets', root / 'runtime', root / 'target'
            files = {
                'harn/agent.harn': b'pipeline fixture', 'harn/launcher.py': b'launcher fixture',
                'harn/skills.patch': b'patch fixture', 'cortex-code/mcp_bridge.py': b'stdio fixture',
            }
            for name, data in files.items():
                path = assets / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            archive = io.BytesIO()
            with tarfile.open(fileobj=archive, mode='w:gz') as package:
                for name in ('LICENSE-MIT', 'LICENSE-APACHE'):
                    data = b'license fixture'
                    member = tarfile.TarInfo(f'{SOURCE}/{name}')
                    member.size = len(data)
                    package.addfile(member, io.BytesIO(data))
                member = tarfile.TarInfo(f'{SOURCE}/../outside' if unsafe_member else f'{SOURCE}/alias')
                member.type = tarfile.SYMTYPE
                member.linkname = '../outside'
                package.addfile(member)
            data = archive.getvalue()
            contract = {
                'version': '0.10.201', 'source_url': 'https://example.invalid/pinned-source',
                'source_sha256': digest(data),
                'patch_sha256': digest(b'wrong' if bad_patch else files['harn/skills.patch']),
                'files': {'harn': digest(files['harn/launcher.py']),
                          'agent.harn': digest(files['harn/agent.harn']),
                          'shiju_stdio.py': digest(files['cortex-code/mcp_bridge.py'])},
            }
            (assets / 'contracts.json').write_text(json.dumps({'harn': contract}))
            binary = b'built engine'

            def execute(args, *, cwd, **kwargs):
                self.assertEqual(cwd, assets / SOURCE)
                self.assertFalse((cwd / 'alias').is_symlink())
                if args[0] == 'cargo':
                    self.assertEqual(args, ['cargo', '+1.99.1', 'build', '--locked', '-j', '1',
                                            '-p', 'harn-cli', '--bin', 'harn'])
                    self.assertEqual(kwargs['env']['CARGO_TARGET_DIR'], str(target))
                    self.assertEqual(kwargs['env']['CARGO_BUILD_BUILD_DIR'], str(target))
                    (target / 'debug').mkdir(parents=True)
                    (target / 'debug/harn').write_bytes(binary)

            with patch.object(sys, 'argv', [str(INSTALLER), str(assets), str(runtime), 'harn']), \
                    patch.dict(os.environ, {'CARGO_TARGET_DIR': str(target)}), \
                    patch('urllib.request.urlopen', return_value=io.BytesIO(data)), \
                    patch('subprocess.run', side_effect=execute) as invoked, \
                    patch('platform.system', return_value='Darwin'), \
                    patch('platform.machine', return_value='arm64'):
                if bad_patch or unsafe_member:
                    with self.assertRaisesRegex(RuntimeError, 'checksum mismatch|Unsafe Harn source entry'):
                        runpy.run_path(str(INSTALLER), run_name='__main__')
                    invoked.assert_not_called()
                    self.assertFalse(runtime.exists())
                    return
                with self.assertRaises(SystemExit) as result:
                    runpy.run_path(str(INSTALLER), run_name='__main__')
                self.assertEqual(result.exception.code, 0)
                self.assertEqual(invoked.call_count, 2)
            self.assertEqual((runtime / 'harn-native').read_bytes(), binary)
            self.assertEqual(json.loads((runtime / 'build-receipt.json').read_text()), {
                **{key: contract[key] for key in ('version', 'source_sha256', 'patch_sha256')},
                'engine_sha256': digest(binary),
            })

    def test_receipt_binds_source_patch_and_engine(self):
        self.install()

    def test_unreviewed_patch_never_runs(self):
        self.install(bad_patch=True)

    def test_traversal_in_archive_is_rejected(self):
        self.install(unsafe_member=True)


if __name__ == '__main__':
    unittest.main()
