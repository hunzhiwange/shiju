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
    def test_environment_preserves_slot_and_unifies_intermediates(self):
        with patch.dict(dev.os.environ, {'CARGO_TARGET_DIR': '/repo/target/rust-slot-3',
                                         'CARGO_BUILD_BUILD_DIR': '/old/shared'}, clear=True):
            env = dev.environment()
            self.assertEqual(env['CARGO_TARGET_DIR'], '/repo/target/rust-slot-3')
            self.assertEqual(env['CARGO_BUILD_BUILD_DIR'], env['CARGO_TARGET_DIR'])

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

    def test_font_macro_patch_is_applied_before_vendor_checksums(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / 'registry' / 'iced_fonts_macros-0.3.0'
            (source / 'src').mkdir(parents=True)
            original = ('        doc_link,\n'
                        '    } = parse_macro_input!(input as Input);\n\n'
                        '    let font_path_str = font_path.value();\n'
                        '    let font_data = std::fs::read(&font_path_str).expect("Failed to read font file");\n'
                        '    let face = Face::parse(&font_data, 0).expect("Failed to parse font");\n\n'
                        '    let mut all_codepoints: Vec<char> = Vec::new();\n')
            (source / 'src/lib.rs').write_text(original)
            (root / 'patches').symlink_to(dev.ROOT / 'patches', target_is_directory=True)
            (root / 'Cargo.lock').write_text('[[package]]\nname = "iced_fonts_macros"\nversion = "0.3.0"\nsource = "registry+https://example.test/index"\nchecksum = "package-digest"\n')
            metadata = {'packages': [{'name': 'iced_fonts_macros', 'version': '0.3.0',
                        'source': 'registry+https://example.test/index',
                        'manifest_path': str(source / 'Cargo.toml')}]}
            real_run = dev.run

            def run(args, **kwargs):
                if args[:2] == ['cargo', 'metadata']:
                    from types import SimpleNamespace
                    return SimpleNamespace(stdout=json.dumps(metadata))
                return real_run(args, **kwargs)

            with patch.object(dev, 'ROOT', root), patch.object(dev, 'run', side_effect=run):
                dev.prepare_vendor(root / 'vendor')
            dest = root / 'vendor/iced_fonts_macros-0.3.0'
            self.assertIn('CARGO_MANIFEST_DIR', (dest / 'src/lib.rs').read_text())
            checksum = json.loads((dest / '.cargo-checksum.json').read_text())
            self.assertEqual(checksum['files']['src/lib.rs'], dev.digest(dest / 'src/lib.rs'))
            self.assertEqual((source / 'src/lib.rs').read_text(), original)

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

    def test_macos_bundle_has_independent_identity_and_updates_runner(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / 'runner'
            source.write_bytes(b'first runner')
            source.chmod(0o755)
            with patch.object(dev, 'STATE', root / 'state'):
                executable = dev.desktop_bundle(source)
                info = dev.plistlib.loads((executable.parents[1] / 'Info.plist').read_bytes())
                self.assertTrue(info['CFBundleIdentifier'].startswith('cloud.shiju.relax-dev.'))
                self.assertEqual(info['CFBundleExecutable'], executable.name)
                self.assertEqual(executable.read_bytes(), b'first runner')
                self.assertTrue(executable.stat().st_mode & 0o111)
                source.write_bytes(b'updated runner')
                self.assertEqual(dev.desktop_bundle(source).read_bytes(), b'updated runner')

    def test_live_test_launch_preserves_filter_and_shared_slot(self):
        with patch.dict(dev.os.environ, {}, clear=True), \
                patch.object(dev, 'prepare', return_value=(Path('/tool/relax'), Path('/tool/source'), Path('/run'))), \
                patch.object(dev, 'desktop_bundle') as bundle, \
                patch.object(dev.os, 'chdir'), patch.object(dev.os, 'execve') as execute:
            self.assertIsNone(dev.main(['test', 'vw-desktop', 'voice_task_11_']))
            _, args, env = execute.call_args.args
            self.assertEqual(env['MAKEPAD_RUN_AHEAD'], 'minimal')
            self.assertEqual(args, ['/tool/relax', 'test', '--live', '--allow-c', '--cfg',
                                    'debug_assertions', 'vw-desktop', '--lib', '--',
                                    'voice_task_11_', '--nocapture', '--test-threads=1'])
            bundle.assert_not_called()

    def test_live_test_honors_explicit_ahead_policy(self):
        with patch.dict(dev.os.environ, {'MAKEPAD_RUN_AHEAD': 'balanced'}, clear=True), \
                patch.object(dev, 'prepare', return_value=(Path('/tool/relax'), Path('/tool/source'), Path('/run'))), \
                patch.object(dev.os, 'chdir'), patch.object(dev.os, 'execve') as execute:
            dev.main(['test', 'vw-desktop', 'voice_task_11_'])
            self.assertEqual(execute.call_args.args[2]['MAKEPAD_RUN_AHEAD'], 'balanced')

    def test_live_test_rejects_missing_or_ambiguous_selection_before_setup(self):
        for args in [['test'], ['test', 'vw-desktop'], ['test', 'vw-desktop', ''],
                     ['test', 'vw-desktop', '   '], ['test', '--all', 'voice_'],
                     ['test', 'vw-desktop', '--ignored'],
                     ['test', 'vw-desktop', 'voice_', 'unexpected']]:
            with self.subTest(args=args), patch.object(dev, 'prepare') as prepare:
                self.assertEqual(dev.main(args), 2)
                prepare.assert_not_called()

    def test_desktop_launch_uses_actual_binary_name(self):
        with patch.dict(dev.os.environ, {}, clear=True), \
                patch.object(dev, 'prepare', return_value=(Path('/tool/relax'), Path('/tool/source'), Path('/run'))), \
                patch.object(dev, 'desktop_bundle', return_value=Path('/tool/relax')), \
                patch.object(dev.os, 'chdir'), patch.object(dev.os, 'execve') as execute:
            dev.main(['desktop'])
            self.assertEqual(execute.call_args.args[1][-3:], ['vw-desktop', '--bin', 'shi-ju'])
            self.assertEqual(execute.call_args.args[2]['MAKEPAD_RUN_O3'], '0')
            self.assertNotIn('--unattended', execute.call_args.args[1])

    def test_desktop_honors_explicit_o3_for_compiler_reproduction(self):
        with patch.dict(dev.os.environ, {'MAKEPAD_RUN_O3': '1'}, clear=True), \
                patch.object(dev, 'prepare', return_value=(Path('/tool/relax'), Path('/tool/source'), Path('/run'))), \
                patch.object(dev, 'desktop_bundle', return_value=Path('/tool/relax')), \
                patch.object(dev.os, 'chdir'), patch.object(dev.os, 'execve') as execute:
            dev.main(['desktop'])
            self.assertEqual(execute.call_args.args[2]['MAKEPAD_RUN_O3'], '1')


if __name__ == '__main__':
    unittest.main()
