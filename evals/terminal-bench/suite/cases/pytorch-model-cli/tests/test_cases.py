"""Host behavioral controls; Docker controls remain separately mandatory."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from suite import diagnostics

ROOT = Path(__file__).resolve().parents[1]
CASE = json.loads((ROOT / 'case.json').read_text())


class ModelCliCases(unittest.TestCase):
    def test_case_contract(self):
        diagnostics.load_case(ROOT / 'case.json')


def add_test(child, identity, files, expected):
    def test(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app, oracle = root / 'app', root / 'oracle'
            app.mkdir()
            oracle.mkdir()
            diagnostics.copy_files(ROOT, child['fixtures'], app)
            diagnostics.copy_files(ROOT, files, app)
            diagnostics.copy_files(ROOT, child['oracle'], oracle)
            result = subprocess.run([sys.executable, str(ROOT / 'check.py'), str(app), str(oracle)], capture_output=True, timeout=30)
            self.assertEqual(result.returncode, expected, result.stderr.decode())
    setattr(ModelCliCases, f'test_{child["id"]}_{identity.replace("-", "_")}', test)


for child in CASE['children']:
    for identity, files, expected in [('gold', child['gold'], 0), ('nop', {}, 1)] + [(n['id'], n['files'], 1) for n in child['negatives']]:
        add_test(child, identity, files, expected)

if __name__ == '__main__':
    unittest.main()
