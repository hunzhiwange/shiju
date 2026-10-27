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


    def test_expected_values_follow_independent_specification(self):
        import json
        data = [json.loads((ROOT / f'fixtures/m{i}/input.json').read_text()) for i in range(1, 5)]
        expected = [json.loads((ROOT / f'oracle/m{i}/expected.json').read_text())['expected'] for i in range(1, 5)]
        def parity(value):
            return sum(int(bit) for bit in bin(value)[2:]) % 2
        def rotate(value):
            return (value % 64) * 4 + value // 64
        pairs = [tuple(map(int, line.split())) for line in data[0]['pairs_text'].splitlines()]
        self.assertEqual([(v['plaintext'], v['ciphertext']) for v in expected[0]], pairs)
        for row in expected[0]:
            for prefix, key in [('p', 'plaintext'), ('c', 'ciphertext')]:
                self.assertEqual(row[prefix + '_left'] * 2**32 + row[prefix + '_right'], row[key])
                self.assertTrue(0 <= row[prefix + '_right'] < 2**32)
        x = data[1]
        relations = [parity(p & x['plaintext_mask']) ^ parity(c & x['ciphertext_mask']) for p, c in x['pairs']]
        zeros = len(relations) - sum(relations)
        self.assertEqual(expected[1], dict(relations=relations, zeros=zeros, ones=sum(relations),
                                         bias_numerator=zeros-sum(relations), bias_denominator=2*len(relations)))
        rows = []
        x = data[2]
        for key in x['candidates']:
            zeros = sum(parity(p & x['plaintext_mask']) == parity(rotate((c+key) % 256) & x['output_mask']) for p, c in x['pairs'])
            rows.append(dict(key=key, zeros=zeros, score=abs(2*zeros-len(x['pairs']))))
        self.assertEqual(expected[2], sorted(rows, key=lambda row: (-row['score'], row['key'])))
        x = data[3]
        for plaintext, ciphertext in zip(expected[3]['plaintexts'], x['ciphertexts']):
            left, right = divmod(plaintext, 256)
            for key in x['keys']:
                left, right = right, left ^ rotate((right+key) % 256)
            self.assertEqual(left*256+right, ciphertext)
        self.assertEqual(expected[3]['matches'], [p == q for p, q in zip(expected[3]['plaintexts'], x['known_plaintexts'])])
        self.assertEqual(set(expected[3]['matches']), {True, False})

    def test_replaced_input_symlink_is_rejected(self):
        child = CASE['children'][0]
        def mutate(app):
            p = app / 'solution.py'
            p.write_text(p.read_text() + "\nfrom pathlib import Path\np=Path('input.json');b=p.read_bytes();p.unlink();Path('saved.json').write_bytes(b);p.symlink_to('saved.json')\n")
        self.verify(child, child['gold'], 1, mutate)

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
