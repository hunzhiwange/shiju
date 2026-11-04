"""Exercise the public log-summary CLI using timestamped cargo-dist output."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / 'summarize_release_build.py'
spec = importlib.util.spec_from_file_location('release_summary', SCRIPT)
summary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(summary)


def build(package, start, end=None):
    lines = [f'2026-11-04T00:00:{start:02d}Z building x86_64-apple-darwin target, using cargo profile dist --package={package})',
             f'2026-11-04T00:00:{start + 2:02d}Z    Compiling fixture v1.0.0']
    if end is not None:
        lines.append(f'2026-11-04T00:00:{end:02d}Z     Finished `dist` profile [optimized] target(s) in 10.00s')
    return '\n'.join(lines) + '\n'


class ReleaseBuildSummaryTests(unittest.TestCase):
    def summarize(self, log, exit_code=0):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'build.log'
            path.write_text(log)
            result = subprocess.run([sys.executable, str(SCRIPT), str(path),
                                     '--exit-code', str(exit_code)], capture_output=True, text=True)
            return result, json.loads(result.stdout) if result.stdout else None

    def test_summary_reports_package_durations(self):
        log = ''.join(build(p, i * 12, i * 12 + 10) for i, p in enumerate(
            ['vw-acp', 'vw-cli', 'vw-desktop', 'vw-webview']))
        result, report = self.summarize(log)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([b['seconds'] for b in report['builds']], [10] * 4)
        self.assertEqual(report['builds'][1]['post_compile_seconds'], 8)
        self.assertEqual(report['package_seconds']['vw-cli'], 10)

    def test_summary_rejects_incomplete_build(self):
        result, report = self.summarize(build('vw-cli', 0))
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(report['success'])
        self.assertIsNone(report['builds'][0]['seconds'])

    def test_summary_separates_repeated_builds(self):
        result, report = self.summarize(build('vw-cli', 0, 10) + build('vw-cli', 12, 22))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(report['builds']), 2)
        self.assertEqual(report['package_seconds']['vw-cli'], 20)
        self.assertEqual([b['attempt'] for b in report['builds']], [1, 2])

    def test_failed_exit_cannot_report_success(self):
        result, report = self.summarize(build('vw-cli', 0, 10), 7)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(report['success'])
        self.assertEqual(report['exit_code'], 7)

    def test_new_build_does_not_close_incomplete_attempt(self):
        result, report = self.summarize(build('vw-cli', 0) + build('vw-cli', 12, 22))
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(report['builds'][0]['seconds'])
        self.assertEqual(report['package_seconds']['vw-cli'], 10)

    def test_duplicate_finish_is_not_counted_twice(self):
        result, report = self.summarize(build('vw-cli', 0, 10) +
            '2026-11-04T00:00:11Z     Finished `dist` profile [optimized] target(s) in 10.00s\n')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(report['package_seconds']['vw-cli'], 10)

    def test_empty_log_is_not_success(self):
        result, report = self.summarize('')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(report['success'])


class BenchmarkExecutionTests(unittest.TestCase):
    @unittest.skipIf(os.name == 'nt', 'executable fixture uses a POSIX shebang')
    def test_capture_preserves_failure_code_and_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable = root / 'dist'
            executable.write_text("#!/usr/bin/env python3\nimport sys\n"
                                  "print('{\"fixture\": true}')\n"
                                  "print('error: fixture failed', file=sys.stderr)\n"
                                  "sys.exit(7)\n")
            executable.chmod(0o755)
            output = root / 'report'
            output.mkdir()
            with patch.dict(os.environ, PATH=str(root) + os.pathsep + os.environ['PATH']):
                code = summary.run_dist('x86_64-apple-darwin', output)
            self.assertEqual(code, 7)
            self.assertEqual(json.loads((output / 'dist-manifest.json').read_text()),
                             {'fixture': True})
            report = summary.summarize((output / 'build.log').read_text(), code)
            self.assertFalse(report['success'])
            self.assertEqual(report['exit_code'], 7)
            self.assertIn('fixture failed', (output / 'build.log').read_text())

    def test_slot_skips_occupied_lock_and_releases_on_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            occupied = root / 'target/rust-slot-locks/slot-1.lock'
            occupied.mkdir(parents=True)
            with patch.object(summary, 'ROOT', root), patch.dict(os.environ):
                with self.assertRaisesRegex(RuntimeError, 'fixture'):
                    with summary.slot() as (number, target):
                        self.assertEqual(number, 2)
                        self.assertEqual(os.environ['CARGO_TARGET_DIR'], str(target))
                        self.assertTrue(target.is_absolute())
                        lock = root / 'target/rust-slot-locks/slot-2.lock'
                        self.assertEqual(json.loads((lock / 'owner.json').read_text())['pid'],
                                         os.getpid())
                        target.mkdir()
                        (target / 'cached').touch()
                        raise RuntimeError('fixture')
                self.assertFalse(lock.exists())
                self.assertTrue(occupied.exists())
                self.assertTrue((target / 'cached').exists())


if __name__ == '__main__':
    unittest.main()
