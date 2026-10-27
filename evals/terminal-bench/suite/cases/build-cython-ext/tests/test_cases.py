"""Exercise the actual checker and real compiled extension artifacts."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from suite.diagnostics import run_checker

ROOT = Path(__file__).resolve().parents[1]


class CaseTests(unittest.TestCase):
    def test_report_rejects_meaningful_errors(self):
        child = json.loads((ROOT / 'case.json').read_text())['children'][0]
        for name, files, expected in [('gold', child['gold'], 0), ('nop', {}, 1)] + [
                (n['id'], n['files'], 1) for n in child['negatives']]:
            with self.subTest(candidate=name), tempfile.TemporaryDirectory() as temp:
                app, oracle = Path(temp) / 'app', Path(temp) / 'oracle'
                for mapping, dest in [(child['fixtures'], app), (files, app), (child['oracle'], oracle)]:
                    dest.mkdir(exist_ok=True)
                    for target, source in mapping.items():
                        out = dest / target
                        out.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copyfile(ROOT / source, out)
                result = subprocess.run([sys.executable, str(ROOT / 'check.py'), str(app), str(oracle)],
                                        capture_output=True, timeout=10)
                self.assertEqual(result.returncode, expected, result.stderr.decode())

    def test_native_gold_nop_and_three_errors_per_child(self):
        case = json.loads((ROOT / 'case.json').read_text())
        for child in case['children'][1:]:
            for name, files, expected in [('gold', child['gold'], 0), ('nop', {}, 1)] + [
                    (n['id'], n['files'], 1) for n in child['negatives']]:
                with self.subTest(child=child['id'], candidate=name):
                    self.assertEqual(run_checker(case['image'], ROOT, child, files), expected)
