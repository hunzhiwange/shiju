from pathlib import Path
import json
import shutil
import tempfile
import unittest
from suite import diagnostics

ROOT = Path(__file__).resolve().parents[1]

class CaseTests(unittest.TestCase):
    def test_architecture_selection_rejects_other_operating_system(self):
        case, _ = diagnostics.load_case(ROOT / 'case.json')
        child = case['children'][0]
        negative = next(n for n in child['negatives'] if n['id'] == 'accepts-other-os')
        self.assertEqual(diagnostics.run_checker(case['image'], ROOT, child, negative['files']), 1)

    def test_gold_nop_and_three_mutations_per_child(self):
        case, _ = diagnostics.load_case(ROOT / 'case.json')
        for child in case['children']:
            candidates = [('gold', child['gold'], 0), ('nop', {}, 1)]
            candidates += [(n['id'], n['files'], 1) for n in child['negatives']]
            for name, files, expected in candidates:
                with self.subTest(child=child['id'], candidate=name):
                    self.assertEqual(diagnostics.run_checker(case['image'], ROOT, child, files), expected)

    def test_modified_public_driver_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'case'
            shutil.copytree(ROOT, root)
            case, _ = diagnostics.load_case(root / 'case.json')
            child = case['children'][1]
            driver = root / child['fixtures']['driver.c']
            driver.write_text(driver.read_text() + '\n/* changed public input */\n')
            self.assertEqual(diagnostics.run_checker(case['image'], root, child, child['gold']), 1)
