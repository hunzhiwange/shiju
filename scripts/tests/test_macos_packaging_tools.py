import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'install_macos_packaging_tools.sh'


class PackagingToolsTests(unittest.TestCase):
    def run_case(self, version='', failure=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'bin').mkdir()
            cargo = root / 'bin/cargo'
            cargo.write_text('#!/bin/bash\nprintf "%s\\n" "$*" >> "$CALLS"\nif [[ "$1" == install ]]; then\n  [[ "$FAIL" == 1 ]] && exit 7\n  mkdir -p "$TOOLS/bin"\n  printf \'#!/bin/bash\\necho "cargo-bundle v0.12.0"\\n\' > "$TOOLS/bin/cargo-bundle"\n  chmod +x "$TOOLS/bin/cargo-bundle"\nfi\n')
            cargo.chmod(0o755)
            tools = root / 'tools'
            if version:
                (tools / 'bin').mkdir(parents=True)
                binary = tools / 'bin/cargo-bundle'
                binary.write_text(f'#!/bin/bash\necho "cargo-bundle v{version}"\n')
                binary.chmod(0o755)
            calls = root / 'calls'
            result = subprocess.run(['bash', str(SCRIPT), str(tools)], env={**os.environ, 'PATH': f'{root}/bin:' + os.environ['PATH'], 'TOOLS': str(tools), 'CALLS': str(calls), 'FAIL': str(int(failure)), 'CARGO_TARGET_DIR': str(root / 'target')}, capture_output=True, text=True)
            return result, calls.read_text() if calls.exists() else ''

    def test_matching_version_skips_install(self):
        result, calls = self.run_case('0.12.0')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls, '')

    def test_missing_or_wrong_version_installs_pinned_version(self):
        for version in ('', '0.11.0'):
            with self.subTest(version=version):
                result, calls = self.run_case(version)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn('install cargo-bundle --version 0.12.0 --locked --force --root ', calls)

    def test_install_failure_propagates(self):
        result, _ = self.run_case(failure=True)
        self.assertEqual(result.returncode, 7)
