"""Independent controls execute candidate artifacts and real compiled programs."""
from pathlib import Path
import unittest
from suite.diagnostics import load_case, run_checker

ROOT = Path(__file__).resolve().parents[1]

class Cases(unittest.TestCase):
    def test_case_contract(self):
        case, hashes = load_case(ROOT / 'case.json')
        self.assertEqual(len(case['children']), 4)
        self.assertGreater(len(hashes), 20)

    def test_gold_nop_and_meaningful_wrong_variants(self):
        case, _ = load_case(ROOT / 'case.json')
        for child in case['children']:
            controls = [('gold', child['gold'], 0), ('nop', {}, 1)]
            controls += [(n['id'], n['files'], 1) for n in child['negatives']]
            for name, files, expected in controls:
                with self.subTest(child=child['id'], control=name):
                    self.assertEqual(run_checker(case['image'], ROOT, child, files), expected)

    def test_protected_mutation_and_output_symlink_are_rejected(self):
        import tempfile
        from suite.diagnostics import copy_files
        import importlib.util
        spec = importlib.util.spec_from_file_location('heap_check', ROOT / 'check.py')
        checker = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(checker)
        case, _ = load_case(ROOT / 'case.json')
        for child in case['children']:
            with tempfile.TemporaryDirectory() as directory:
                app = Path(directory) / 'app'; app.mkdir()
                oracle = Path(directory) / 'oracle'
                copy_files(ROOT, child['fixtures'], app)
                copy_files(ROOT, child['gold'], app)
                copy_files(ROOT, child['oracle'], oracle)
                protected = next(n for n in child['fixtures'] if n != 'user.cpp')
                (app / protected).write_text('tampered')
                self.assertFalse(checker.check(app, oracle))
            if child['id'] == 'm2':
                with tempfile.TemporaryDirectory() as directory:
                    app = Path(directory) / 'app'; app.mkdir()
                    oracle = Path(directory) / 'oracle'
                    copy_files(ROOT, child['fixtures'], app)
                    copy_files(ROOT, child['oracle'], oracle)
                    (app / 'diagnosis.json').symlink_to(ROOT / child['gold']['diagnosis.json'])
                    self.assertFalse(checker.check(app, oracle))

    def test_diagnosis_rejects_duplicate_keys_boolean_ids_and_extra_fields(self):
        import tempfile
        import importlib.util
        import json
        from suite.diagnostics import copy_files
        spec = importlib.util.spec_from_file_location('heap_check_json', ROOT / 'check.py')
        checker = importlib.util.module_from_spec(spec); spec.loader.exec_module(checker)
        case, _ = load_case(ROOT / 'case.json'); child = case['children'][1]
        gold = json.loads((ROOT / child['gold']['diagnosis.json']).read_text())
        wrong = [json.dumps([dict(gold[0], allocation_event=True)]),
                 json.dumps([dict(gold[0], explanation='guessed')]),
                 '[{"object":"cache","object":"cache","arena":"alpha","allocation_event":12,"destruction_event":17,"release_event":18}]']
        for content in wrong:
            with self.subTest(content=content), tempfile.TemporaryDirectory() as directory:
                app = Path(directory) / 'app'; app.mkdir()
                oracle = Path(directory) / 'oracle'
                copy_files(ROOT, child['fixtures'], app)
                copy_files(ROOT, child['oracle'], oracle)
                (app / 'diagnosis.json').write_text(content)
                self.assertFalse(checker.check(app, oracle))
