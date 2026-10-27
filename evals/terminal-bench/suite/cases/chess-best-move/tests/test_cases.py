"""Exercise actual candidate files against the evaluator in a separate process."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from suite.diagnostics import copy_files, load_case

ROOT = Path(__file__).resolve().parents[1]


class Cases(unittest.TestCase):
    def run_checker(self, child, files=None, text=None):
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory) / 'app'
            oracle = Path(directory) / 'oracle'
            app.mkdir()
            oracle.mkdir()
            copy_files(ROOT, child['fixtures'], app)
            copy_files(ROOT, files or {}, app)
            copy_files(ROOT, child['oracle'], oracle)
            if text is not None:
                task = json.loads((oracle / 'task.json').read_text())
                (app / task['output']).write_text(text)
            return subprocess.run(
                [sys.executable, str(ROOT / 'check.py'), str(app), str(oracle)],
                capture_output=True, timeout=10)

    def test_gold_and_all_negative_artifacts(self):
        case, _ = load_case(ROOT / 'case.json')
        for child in case['children']:
            candidates = [('gold', child['gold'], 0), ('nop', {}, 1)]
            candidates += [(n['id'], n['files'], 1) for n in child['negatives']]
            for name, files, expected in candidates:
                with self.subTest(child=child['id'], artifact=name):
                    actual = self.run_checker(child, files)
                    self.assertEqual(actual.returncode, expected, actual.stderr.decode())

    def test_malformed_and_wrongly_typed_artifacts_are_rejected(self):
        case, _ = load_case(ROOT / 'case.json')
        controls = [(0, '{}'), (1, '["b5","c2","c6","e2","e6","f3","f5","f5"]'),
                    (2, '{'), (2, 'null'), (3, 'a2a4\ne1g1\nh7h8q'),
                    (0, '{"a8":"black king","a8":"black king","h1":"white king",'
                     '"c6":"white knight","f3":"black rook"}')]
        task = json.loads((ROOT / 'oracle/m3/task.json').read_text())
        task['expected']['promotion-check']['legal'] = 1
        controls.append((2, json.dumps(task['expected'])))
        for index, text in controls:
            with self.subTest(child=index + 1, text=text):
                actual = self.run_checker(case['children'][index], text=text)
                self.assertEqual(actual.returncode, 1, actual.stderr.decode())
