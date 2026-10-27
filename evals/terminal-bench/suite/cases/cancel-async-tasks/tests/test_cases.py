from pathlib import Path
import unittest
from suite import diagnostics
ROOT = Path(__file__).resolve().parents[1]
class CaseTests(unittest.TestCase):
    def test_gold_nop_and_three_mutations_per_child(self):
        case, _ = diagnostics.load_case(ROOT / 'case.json')
        for child in case['children']:
            candidates = [('gold', child['gold'], 0), ('nop', {}, 1)]
            candidates += [(n['id'], n['files'], 1) for n in child['negatives']]
            for name, files, expected in candidates:
                with self.subTest(child=child['id'], candidate=name):
                    self.assertEqual(diagnostics.run_checker(case['image'], ROOT, child, files), expected)
