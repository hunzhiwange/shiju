"""Run the real checker against synthetic artifacts, never model prose."""
import json
import importlib.util
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class CaseTests(unittest.TestCase):
    def test_gold_nop_and_meaningful_errors(self):
        case = json.loads((ROOT / 'case.json').read_text())
        for child in case['children']:
            for name, files, expected in [('gold', child['gold'], 0), ('nop', {}, 1)] + [
                    (n['id'], n['files'], 1) for n in child['negatives']]:
                with self.subTest(child=child['id'], candidate=name), tempfile.TemporaryDirectory() as temp:
                    app, oracle = Path(temp) / 'app', Path(temp) / 'oracle'
                    for mapping, dest in [(child['fixtures'], app), (files, app), (child['oracle'], oracle)]:
                        dest.mkdir(exist_ok=True)
                        for target, source in mapping.items():
                            out = dest / target
                            out.parent.mkdir(parents=True, exist_ok=True)
                            shutil.copyfile(ROOT / source, out)
                    result = subprocess.run([sys.executable, str(ROOT / child['checker']), str(app), str(oracle)],
                                            capture_output=True, timeout=10)
                    self.assertEqual(result.returncode, expected, result.stderr.decode())

    def test_numeric_serialization_is_semantic_and_rejects_nonfinite(self):
        spec = importlib.util.spec_from_file_location('bn_checker', ROOT / 'check.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        self.assertTrue(module.same_json({'intercept': 2.0}, {'intercept': 2}))
        self.assertFalse(module.same_json({'intercept': True}, {'intercept': 1}))
        self.assertFalse(module.same_json({'intercept': float('nan')}, {'intercept': 2}))
        self.assertFalse(module.same_json({'intercept': float('inf')}, {'intercept': 2}))
