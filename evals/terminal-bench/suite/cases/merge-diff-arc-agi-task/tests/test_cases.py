"""Real submitted programs, isolated synthetic bundles, and negative controls."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from suite import diagnostics

ROOT = Path(__file__).resolve().parents[1]


class MergeDiffCases(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.case, _ = diagnostics.load_case(ROOT / 'case.json')


def add_test(child, identity, expected):
    def test(self):
        files = (child['gold'] if identity == 'gold' else {} if identity == 'nop' else
                 next(n['files'] for n in child['negatives'] if n['id'] == identity))
        with tempfile.TemporaryDirectory(prefix='synthetic merge ') as directory:
            root = Path(directory)
            app, oracle = root / 'app', root / 'oracle'
            app.mkdir()
            diagnostics.copy_files(ROOT, child['fixtures'], app)
            diagnostics.copy_files(ROOT, files, app)
            diagnostics.copy_files(ROOT, child['oracle'], oracle)
            result = subprocess.run([sys.executable, str(ROOT / 'check.py'), str(app), str(oracle)],
                                    capture_output=True, timeout=30,
                                    env={**os.environ, 'GIT_CONFIG_GLOBAL': '/dev/null',
                                         'GIT_CONFIG_NOSYSTEM': '1'})
            self.assertEqual(result.returncode, expected, result.stderr.decode())
    setattr(MergeDiffCases, f'test_{child["id"]}_{identity.replace("-", "_")}', test)


for child in json.loads((ROOT / 'case.json').read_text())['children']:
    for identity, expected in [('gold', 0), ('nop', 1)] + [(n['id'], 1) for n in child['negatives']]:
        add_test(child, identity, expected)


if __name__ == '__main__':
    unittest.main()
