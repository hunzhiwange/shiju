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
                            lambda app: (app / output).write_text('null'))

    def test_symlink_outputs_are_rejected(self):
        for child in CASE['children']:
            output = next(iter(child['gold']))
            def mutate(app):
                target = app / output
                target.rename(app / 'saved')
                target.symlink_to(app / 'saved')
            with self.subTest(child=child['id']):
                self.verify(child, child['gold'], 1, mutate)

    def test_gold_matches_binary_specification(self):
        import json
        import struct
        def expected(n):
            return json.loads((ROOT / f'oracle/m{n}/expected.json').read_text())['expected']
        header = (ROOT / 'fixtures/m1/header.bin').read_bytes()
        self.assertEqual(header[:7], b'\x7fELF\x02\x01\x01')
        self.assertEqual(expected(1), {'class': 64, 'byte_order': 'little',
            'entry': struct.unpack_from('<Q', header, 24)[0],
            'section_offset': struct.unpack_from('<Q', header, 40)[0],
            'section_entry_size': struct.unpack_from('<H', header, 58)[0],
            'section_count': struct.unpack_from('<H', header, 60)[0],
            'name_table_index': struct.unpack_from('<H', header, 62)[0]})
        layout = json.loads((ROOT / 'fixtures/m2/layout.json').read_text())
        self.assertEqual(expected(2), [next((r['offset'] + q - r['address']
            for r in layout['regions'] if r['address'] <= q < r['address'] + r['size']), None)
            for q in layout['queries']])
        data = (ROOT / 'fixtures/m3/data.bin').read_bytes()
        regions = json.loads((ROOT / 'fixtures/m3/regions.json').read_text())
        self.assertEqual(expected(3), [[int.from_bytes(data[i:i+4], r['byte_order'])
            for i in range(r['offset'], min(len(data), r['offset']+r['size'])-3, 4)] for r in regions])
        words = json.loads((ROOT / 'fixtures/m4/words.json').read_text())
        self.assertEqual(expected(4), {str(r['address']): int.from_bytes(bytes(r['bytes']), 'little') for r in words})

    def test_duplicate_json_keys_are_rejected(self):
        child = CASE['children'][3]
        self.verify(child, child['gold'], 1, lambda app:
            (app / 'result.json').write_text(
                '{"4198400":0,"4198400":4294967295,"4198404":2147483648,"6291456":305419896}'))

    def test_boolean_is_not_integer(self):
        child = CASE['children'][0]
        self.verify(child, child['gold'], 1, lambda app:
            (app / 'result.json').write_text(
                (app / 'result.json').read_text().replace('"class": 64', '"class": true')))

    def test_json_whitespace_is_accepted(self):
        import json
        for child in CASE['children']:
            self.verify(child, child['gold'], 0, lambda app:
                (app / 'result.json').write_text(json.dumps(json.loads((app / 'result.json').read_text()))))


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
