"""Host black-box tests; Linux fixture calibration remains a separate gate."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from suite.diagnostics import copy_files, load_case

ROOT = Path(__file__).resolve().parents[1]


class Cases(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.case, _ = load_case(ROOT / 'case.json')


def add_test(child_id, identity, expected):
    def test(self):
        child = next(c for c in self.case['children'] if c['id'] == child_id)
        files = child['gold'] if identity == 'gold' else {} if identity == 'nop' else next(n['files'] for n in child['negatives'] if n['id'] == identity)
        with tempfile.TemporaryDirectory() as directory:
            app, oracle = Path(directory) / 'app', Path(directory) / 'oracle'
            app.mkdir()
            oracle.mkdir()
            copy_files(ROOT, child['fixtures'], app)
            copy_files(ROOT, files, app)
            copy_files(ROOT, child['oracle'], oracle)
            result = subprocess.run([sys.executable, '-B', str(ROOT / 'check.py'), str(app), str(oracle)],
                                    capture_output=True, timeout=110)
            self.assertEqual(result.returncode, expected, result.stderr.decode())
    setattr(Cases, f'test_{child_id}_{identity.replace("-", "_")}', test)


for child in json.loads((ROOT / 'case.json').read_text())['children']:
    for identity, expected in [('gold', 0), ('nop', 1)] + [(n['id'], 1) for n in child['negatives']]:
        add_test(child['id'], identity, expected)

if __name__ == '__main__':
    unittest.main()
