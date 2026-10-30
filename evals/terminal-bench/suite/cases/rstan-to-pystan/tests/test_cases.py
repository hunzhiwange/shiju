import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

from suite.diagnostics import load_case, materialize, run_checker

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('gp_checker', ROOT / 'check.py')
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class CaseTests(unittest.TestCase):
    def test_means_gold_writes_numeric_csv_rows(self):
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory)
            shutil.copyfile(ROOT / 'oracle/m4/gold/solution.py', app / 'solution.py')
            checker.check_means(app, app)

    def test_extra_target_column_is_rejected(self):
        program = (ROOT / 'oracle/m1/gold/solution.py').read_text().replace(
            'set(reader.fieldnames)!=set(names)',
            'not set(names).issubset(reader.fieldnames)')
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory)
            (app / 'solution.py').write_text(program)
            with self.assertRaises(checker.Reject):
                checker.check_data(app, app)

    def test_data_and_initials_gold_reject_semantic_mutations(self):
        for child, check in [('m1', checker.check_data), ('m3', checker.check_initials)]:
            for fixture in sorted((ROOT / 'oracle' / child).iterdir()):
                if not fixture.is_dir():
                    continue
                with self.subTest(child=child, fixture=fixture.name), tempfile.TemporaryDirectory() as directory:
                    app = Path(directory)
                    shutil.copyfile(fixture / 'solution.py', app / 'solution.py')
                    if fixture.name == 'gold':
                        check(app, app)
                    else:
                        with self.assertRaises(checker.Reject):
                            check(app, app)

    def test_gold_nop_and_three_negatives_in_real_tool_image(self):
        case, _ = load_case(ROOT / 'case.json')
        count = 0
        for child in case['children']:
            controls = [('gold', child['gold'], 0), ('nop', {}, 1)]
            controls += [(n['id'], n['files'], 1) for n in child['negatives']]
            for name, files, expected in controls:
                with self.subTest(child=child['id'], fixture=name):
                    self.assertEqual(run_checker(case['image'], ROOT, child, files,
                                                 parent_id=case['parent_id']), expected)
                    count += 1
        self.assertEqual(count, 20)

    def test_materialized_tasks_keep_budget_and_hidden_assets_separate(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tasks'
            materialize(ROOT / 'case.json', output)
            import tomllib
            for task in output.iterdir():
                config = tomllib.loads((task / 'task.toml').read_text())
                self.assertEqual(config['environment']['memory_mb'], 2048)
                self.assertEqual(config['agent']['timeout_sec'], 900)
                self.assertEqual(config['verifier']['timeout_sec'], 120)
                visible = {p.relative_to(task / 'environment').as_posix()
                           for p in (task / 'environment').rglob('*') if p.is_file()}
                self.assertEqual(visible, {'Dockerfile', 'fixtures/example.json'})
                self.assertFalse((task / 'environment/solution.py').exists())


if __name__ == '__main__':
    unittest.main()
