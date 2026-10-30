"""Host-only behavioral evidence; container calibration is tracked separately."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from suite import diagnostics

ROOT = Path(__file__).resolve().parents[1]
CASE = json.loads((ROOT/'case.json').read_text())


class RecoveryPrograms(unittest.TestCase):
    def test_schema(self):
        diagnostics.load_case(ROOT/'case.json')


def add_test(child, identity, files, expected):
    def test(self):
        with tempfile.TemporaryDirectory() as directory:
            app, oracle = Path(directory)/'app', Path(directory)/'oracle'
            for mapping, dest in [(child['fixtures'], app), (files, app), (child['oracle'], oracle)]:
                dest.mkdir(exist_ok=True)
                for target, source in mapping.items():
                    shutil.copyfile(ROOT/source, dest/target)
            result = subprocess.run([sys.executable, str(ROOT/'check.py'), str(app), str(oracle)],
                                    capture_output=True, text=True, timeout=50)
            self.assertEqual(result.returncode, expected, result.stderr)
    setattr(RecoveryPrograms, f"test_{child['id']}_{identity.replace('-', '_')}", test)


for child in CASE['children']:
    for identity, files, expected in [('gold', child['gold'], 0), ('nop', {}, 1)] + [(n['id'], n['files'], 1) for n in child['negatives']]:
        add_test(child, identity, files, expected)
