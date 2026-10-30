"""Run real programs, wheels, HTTP services and pip; no Docker required."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from suite.diagnostics import copy_files, load_case

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('pypi_checker', ROOT / 'check.py')
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class CaseTests(unittest.TestCase):
    def test_schema_and_isolation(self):
        case, hashes = load_case(ROOT / 'case.json')
        self.assertEqual(len(case['children']), 4)
        self.assertGreater(len(hashes), 30)

    def run_child(self, index):
        child = json.loads((ROOT / 'case.json').read_text())['children'][index]
        candidates = [('gold', child['gold'], True), ('nop', {}, False)]
        candidates += [(n['id'], n['files'], False) for n in child['negatives']]
        for identity, files, expected in candidates:
            with self.subTest(child=child['id'], candidate=identity), tempfile.TemporaryDirectory() as tmp:
                app, oracle = Path(tmp) / 'app', Path(tmp) / 'oracle'
                app.mkdir(); oracle.mkdir()
                copy_files(ROOT, child['fixtures'], app)
                copy_files(ROOT, files, app)
                copy_files(ROOT, child['oracle'], oracle)
                self.assertEqual(checker.check(app, oracle), expected)

    def test_numeric_package(self):
        self.run_child(0)

    def test_distribution_build(self):
        self.run_child(1)

    def test_live_sandbox_index(self):
        self.run_child(2)

    def test_real_index_install(self):
        self.run_child(3)
