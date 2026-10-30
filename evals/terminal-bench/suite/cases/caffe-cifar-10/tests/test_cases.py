"""Run each candidate as an independent subprocess against hidden controls."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from suite.diagnostics import copy_files, load_case

ROOT = Path(__file__).resolve().parents[1]
CASE, _ = load_case(ROOT / 'case.json')


class Cases(unittest.TestCase):
    def test_case_schema_has_four_independent_children(self):
        self.assertEqual([c['id'] for c in CASE['children']], ['m1', 'm2', 'm3', 'm4'])


def control(child, files, expected):
    def test(self):
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory) / 'app'
            oracle = Path(directory) / 'oracle'
            app.mkdir()
            oracle.mkdir()
            copy_files(ROOT, child['fixtures'], app)
            copy_files(ROOT, files, app)
            copy_files(ROOT, child['oracle'], oracle)
            result = subprocess.run(
                [sys.executable, str(ROOT / 'check.py'), str(app), str(oracle)],
                capture_output=True, timeout=60)
            self.assertEqual(result.returncode, expected, result.stderr.decode())
    return test


for child in CASE['children']:
    for name, files, expected in [('gold', child['gold'], 0), ('nop', {}, 1)] + [
            (n['id'], n['files'], 1) for n in child['negatives']]:
        setattr(Cases, 'test_' + child['id'] + '_' + name.replace('-', '_'), control(child, files, expected))
