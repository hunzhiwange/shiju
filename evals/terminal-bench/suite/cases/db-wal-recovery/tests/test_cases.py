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

    def test_bool_is_not_frame_integer(self):
        child = CASE['children'][2]
        self.verify(child, child['gold'], 1, lambda app:
                    (app / 'result.json').write_text(
                        (app / 'result.json').read_text().replace('"last_commit_frame": 1', '"last_commit_frame": true')))

    def test_duplicate_json_keys_are_rejected(self):
        child = CASE['children'][2]
        self.verify(child, child['gold'], 1, lambda app:
                    (app / 'result.json').write_text(
                        (app / 'result.json').read_text().replace('{', '{"header_valid":false,', 1)))

    def test_json_whitespace_is_accepted(self):
        import json
        child = CASE['children'][0]
        self.verify(child, child['gold'], 0, lambda app:
                    (app / 'result.json').write_text(json.dumps(json.loads((app / 'result.json').read_text()), sort_keys=True)))

    def test_synthetic_wal_recovers_with_sqlite(self):
        import sqlite3
        import json
        for child, key in [(CASE['children'][1], 0x29), (CASE['children'][3], 0x6d)]:
            with self.subTest(child=child['id']), tempfile.TemporaryDirectory() as directory:
                work = Path(directory)
                (work / 'work.db').write_bytes((ROOT / child['fixtures']['base.db']).read_bytes())
                encrypted = (ROOT / child['fixtures']['changes.xor']).read_bytes()
                (work / 'work.db-wal').write_bytes(bytes(x ^ key for x in encrypted))
                with sqlite3.connect(work / 'work.db') as con:
                    rows = [dict(zip(('id','name','value'), row)) for row in con.execute('SELECT id,name,value FROM readings ORDER BY id')]
                expected = json.loads((ROOT / child['oracle']['expected.json']).read_text())['expected']
                self.assertEqual(rows, expected)



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
