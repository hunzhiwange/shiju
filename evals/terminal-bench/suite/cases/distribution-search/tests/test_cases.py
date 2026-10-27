"""Run real candidate artifacts in checker subprocesses, without model calls."""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from suite.diagnostics import copy_files, load_case

ROOT = Path(__file__).resolve().parents[1]
CASE, _ = load_case(ROOT / 'case.json')


class Cases(unittest.TestCase):
    def verify(self, child, files, expected, mutate=None):
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory) / 'app'
            oracle = Path(directory) / 'oracle'
            app.mkdir()
            oracle.mkdir()
            copy_files(ROOT, child['fixtures'], app)
            copy_files(ROOT, files, app)
            copy_files(ROOT, child['oracle'], oracle)
            if mutate:
                mutate(app)
            result = subprocess.run(
                [sys.executable, str(ROOT / 'check.py'), str(app), str(oracle)],
                capture_output=True, timeout=10)
            self.assertEqual(result.returncode, expected, result.stderr.decode())

    def test_changed_inputs_are_rejected(self):
        for child in CASE['children']:
            with self.subTest(child=child['id']):
                self.verify(child, child['gold'], 1,
                            lambda app: (app / 'input.json').write_text('{}'))

    def test_malformed_outputs_are_rejected(self):
        for child in CASE['children']:
            with self.subTest(child=child['id']):
                output = next(iter(child['gold']))
                self.verify(child, child['gold'], 1,
                            lambda app: (app / output).write_text('null'))


    def test_duplicate_json_keys_are_rejected(self):
        child = CASE['children'][0]
        self.verify(child, child['gold'], 1,
                    lambda app: (app / 'result.json').write_text(
                        '{"forward": 0, "forward": 0, "backward": 0}'))

    def test_nonfinite_and_boolean_metrics_are_rejected(self):
        child = CASE['children'][0]
        for value in ('NaN', 'Infinity', 'true'):
            with self.subTest(value=value):
                self.verify(child, child['gold'], 1,
                            lambda app: (app / 'result.json').write_text(
                                '{"forward": ' + value + ', "backward": 0}'))

    def test_permuted_distribution_is_accepted(self):
        child = CASE['children'][1]
        self.verify(child, child['gold'], 0,
                    lambda app: (app / 'result.json').write_text(
                        '{"probabilities": [0.1, 0.7, 0.2]}'))


def artifact_test(child, files, expected):
    def test(self):
        self.verify(child, files, expected)
    return test


for child in CASE['children']:
    candidates = [('gold', child['gold'], 0), ('nop', {}, 1)]
    candidates += [(n['id'], n['files'], 1) for n in child['negatives']]
    for name, files, expected in candidates:
        setattr(Cases, 'test_' + child['id'] + '_' + name.replace('-', '_'),
                artifact_test(child, files, expected))
