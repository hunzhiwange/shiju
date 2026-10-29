"""Exercise artifacts using the same real isolated browser as calibration."""
from pathlib import Path
import unittest
import shutil
import tempfile
from suite.diagnostics import load_case, run_checker
ROOT = Path(__file__).resolve().parents[1]


class CaseTests(unittest.TestCase):
    def test_gold_nop_and_meaningful_errors(self):
        case, _ = load_case(ROOT / 'case.json')
        for child in case['children']:
            candidates = [('gold', child['gold'], 0), ('nop', {}, 1)]
            candidates += [(n['id'], n['files'], 1) for n in child['negatives']]
            for name, files, expected in candidates:
                with self.subTest(child=child['id'], fixture=name):
                    self.assertEqual(run_checker(case['image'], ROOT, child, files), expected)

    def test_modified_filter_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'case'
            shutil.copytree(ROOT, root)
            case, _ = load_case(root / 'case.json')
            child = case['children'][0]
            (root / 'changed-filter.py').write_text('print("bypassed")\n')
            files = {**child['gold'], 'filter.py': 'changed-filter.py'}
            self.assertEqual(run_checker(case['image'], root, child, files), 1)
