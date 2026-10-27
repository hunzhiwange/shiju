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
                            lambda app: (app / next(iter(child['fixtures']))).write_text('{}'))

    def test_malformed_outputs_are_rejected(self):
        for child in CASE['children']:
            with self.subTest(child=child['id']):
                output = next(iter(child['gold']))
                self.verify(child, child['gold'], 1,
                            lambda app: (app / output).write_text('raise RuntimeError()'))

    def test_symlink_outputs_are_rejected(self):
        for child in CASE['children']:
            output = next(iter(child['gold']))
            def mutate(app):
                target = app / output
                target.rename(app / 'saved')
                target.symlink_to(app / 'saved')
            with self.subTest(child=child['id']):
                self.verify(child, child['gold'], 1, mutate)


    def test_gold_matches_independent_specification(self):
        import json
        def reference(x):
            lo, hi = x % 256, x // 256
            t = (lo + hi + 1) % 256
            rotated = (t * 8 + t // 32) % 256
            return rotated * 256 + (lo ^ rotated ^ 167)
        data = [json.loads((ROOT / f'fixtures/m{i}/input.json').read_text()) for i in range(1, 5)]
        expected = [json.loads((ROOT / f'oracle/m{i}/expected.json').read_text())['expected'] for i in range(1, 4)]
        self.assertEqual(expected[0], [reference(x) for x in data[0]['words']])
        self.assertEqual(expected[1], [{'a':x,'b':x ^ data[1]['delta']} for x in data[1]['plaintexts']])
        self.assertEqual(expected[2], [k for k in data[2]['candidates'] if all(
            reference(o['x'] ^ k) ^ reference(o['y'] ^ k) == o['difference']
            for o in data[2]['observations'])])
        self.assertEqual(expected[2], [127, 255])

    def test_m4_query_validation_rejects_invalid_plaintext(self):
        child = CASE['children'][3]
        self.verify(child, child['gold'], 1, lambda app:
            (app / 'solution.py').write_text('def recover(query):\n    return query(-1)\n'))

    def test_m4_requires_integer_key(self):
        child = CASE['children'][3]
        self.verify(child, child['gold'], 1, lambda app:
            (app / 'solution.py').write_text((app / 'solution.py').read_text().replace(
                'return matches[0]', 'return str(matches[0])')))

    def test_output_boolean_is_not_integer(self):
        child = CASE['children'][0]
        self.verify(child, child['gold'], 1, lambda app:
            (app / 'solution.py').write_text('import json;print(json.dumps([True]*10))'))


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
