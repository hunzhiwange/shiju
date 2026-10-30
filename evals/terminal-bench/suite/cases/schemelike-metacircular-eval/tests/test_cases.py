"""Independent real-process controls; no model output or self-authored tests."""
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
        count = 0
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
                            capture_output=True, timeout=60)
                        self.assertEqual(result.returncode, expected, result.stderr.decode())
                        count += 1
        self.assertEqual(count, 20)
