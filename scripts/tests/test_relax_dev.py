"""Focused checks for installer integrity and live-launch routing."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('relax_dev', Path(__file__).parents[1] / 'relax_dev.py')
dev = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(dev)


class RelaxDevTests(unittest.TestCase):
    def test_bad_patch_stops_before_network_or_build(self):
        with patch.object(dev, 'digest', return_value='invalid'), patch.object(dev, 'run') as run:
            with self.assertRaisesRegex(RuntimeError, '补丁校验失败'):
                dev.prepare_locked()
            run.assert_not_called()

    def test_corrupt_cached_binary_is_rejected(self):
        spec = json.loads(dev.MANIFEST.read_text())
        key = dev.hashlib.sha256((spec['base_commit'] + spec['patch_sha256'] + spec['dev_vendor_patch_sha256'] + 'rustc').encode()).hexdigest()[:20]
        with tempfile.TemporaryDirectory() as folder:
            state = Path(folder)
            install = state / 'tools' / key
            install.mkdir(parents=True)
            (install / 'receipt.json').write_text('{"sha256":"expected"}')
            with patch.object(dev, 'STATE', state), patch.object(dev, 'run') as run:
                run.return_value.stdout = 'rustc'
                with self.assertRaisesRegex(RuntimeError, '安装损坏'):
                    dev.prepare_locked()
                self.assertEqual(run.call_count, 1)

    def test_vendor_keeps_git_and_registry_duplicates_separate(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / 'registry-package'
            source.mkdir()
            (source / 'Cargo.toml').write_text('[package]\nname="same"\nversion="1.0.0"')
            (root / 'Cargo.lock').write_text('[[package]]\nname = "same"\nversion = "1.0.0"\nsource = "registry+https://example.test/index"\nchecksum = "package-digest"\n')
            metadata = {'packages': [
                {'name': 'same', 'version': '1.0.0', 'source': 'registry+https://example.test/index', 'manifest_path': str(source / 'Cargo.toml')},
                {'name': 'same', 'version': '1.0.0', 'source': 'git+https://example.test/repo', 'manifest_path': '/git/Cargo.toml'}]}
            with patch.object(dev, 'ROOT', root), patch.object(dev, 'run') as run:
                run.return_value.stdout = json.dumps(metadata)
                dev.prepare_vendor(root / 'vendor')
            checksum = json.loads((root / 'vendor/same-1.0.0/.cargo-checksum.json').read_text())
            self.assertEqual(checksum['package'], 'package-digest')
            self.assertEqual(checksum['files']['Cargo.toml'], dev.digest(source / 'Cargo.toml'))

    def test_gateway_launch_keeps_live_mode_and_forwards_arguments(self):
        with patch.object(dev, 'prepare', return_value=(Path('/tool/relax'), Path('/tool/source'), Path('/run'))), \
                patch.object(dev.os, 'chdir') as chdir, patch.object(dev.os, 'execve') as execute:
            dev.main(['gateway', '--port', '54771'])
            binary, args, env = execute.call_args.args
            self.assertEqual(binary, Path('/tool/relax'))
            self.assertEqual(args[-4:], ['--', 'gateway', '--port', '54771'])
            self.assertNotIn('--unattended', args)
            self.assertEqual(env['MAKEPAD_RUN_STD'], '/tool/source/libs/relax/compiler/std')
            chdir.assert_called_once_with(Path('/run'))

    def test_desktop_launch_uses_actual_binary_name(self):
        with patch.object(dev, 'prepare', return_value=(Path('/tool/relax'), Path('/tool/source'), Path('/run'))), \
                patch.object(dev.os, 'chdir'), patch.object(dev.os, 'execve') as execute:
            dev.main(['desktop'])
            self.assertEqual(execute.call_args.args[1][-3:], ['vw-desktop', '--bin', 'shi-ju'])


if __name__ == '__main__':
    unittest.main()
