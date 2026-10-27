"""Run real submissions; negative controls demonstrate observable semantic errors."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from suite.diagnostics import copy_files, load_case

ROOT = Path(__file__).resolve().parents[1]


class Cases(unittest.TestCase):
    def test_gold_nop_and_three_semantic_negatives_per_child(self):
        case, _ = load_case(ROOT / 'case.json')
        for child in case['children']:
            candidates = [('gold', child['gold'], 0), ('nop', {}, 1)]
            candidates += [(n['id'], n['files'], 1) for n in child['negatives']]
            for name, files, expected in candidates:
                with self.subTest(child=child['id'], artifact=name):
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
                            capture_output=True, timeout=40)
                        self.assertEqual(result.returncode, expected, result.stderr.decode())

    def test_malformed_artifacts_fail_closed(self):
        import json
        case, _ = load_case(ROOT / 'case.json')
        for child in case['children']:
            with self.subTest(child=child['id']):
                with tempfile.TemporaryDirectory() as directory:
                    app = Path(directory) / 'app'
                    oracle = Path(directory) / 'oracle'
                    app.mkdir()
                    oracle.mkdir()
                    copy_files(ROOT, child['fixtures'], app)
                    copy_files(ROOT, child['oracle'], oracle)
                    task = json.loads((oracle / 'task.json').read_text())
                    (app / task['output']).write_text('not a valid submission\n')
                    result = subprocess.run(
                        [sys.executable, str(ROOT / 'check.py'), str(app), str(oracle)],
                        capture_output=True, timeout=10)
                    self.assertEqual(result.returncode, 1, result.stderr.decode())

    def test_report_rejects_integer_boolean_and_duplicate_json_keys(self):
        import json
        case, _ = load_case(ROOT / 'case.json')
        child = case['children'][3]
        gold = (ROOT / child['gold']['comparison.json']).read_text()
        report = json.loads(gold)
        report[0]['equal'] = 1
        for text in (json.dumps(report), gold.replace('"n": 0', '"n": 0, "n": 0')):
            with self.subTest(text=text):
                with tempfile.TemporaryDirectory() as directory:
                    app = Path(directory) / 'app'
                    oracle = Path(directory) / 'oracle'
                    app.mkdir()
                    oracle.mkdir()
                    copy_files(ROOT, child['fixtures'], app)
                    copy_files(ROOT, child['oracle'], oracle)
                    (app / 'comparison.json').write_text(text)
                    result = subprocess.run(
                        [sys.executable, str(ROOT / 'check.py'), str(app), str(oracle)],
                        capture_output=True, timeout=10)
                    self.assertEqual(result.returncode, 1, result.stderr.decode())
