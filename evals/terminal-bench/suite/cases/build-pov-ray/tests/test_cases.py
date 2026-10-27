"""Execute each submitted program in a fresh restricted offline container."""
from pathlib import Path
import unittest
from suite import diagnostics

CASE = Path(__file__).resolve().parents[1] / 'case.json'


class DiagnosticControls(unittest.TestCase):
    def test_real_artifacts_accept_gold_and_reject_nop_and_mutations(self):
        case, _ = diagnostics.load_case(CASE)
        for child in case['children']:
            controls = [('gold', child['gold'], 0), ('nop', {}, 1)]
            controls += [(n['id'], n['files'], 1) for n in child['negatives']]
            for name, files, expected in controls:
                with self.subTest(child=child['id'], control=name):
                    self.assertEqual(diagnostics.run_checker(case['image'], CASE.parent,
                                                            child, files), expected)
