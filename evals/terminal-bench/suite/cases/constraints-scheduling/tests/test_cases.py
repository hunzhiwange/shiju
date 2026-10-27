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

    def test_ics_semantics_allow_crlf_and_different_uid(self):
        child = CASE['children'][3]
        def mutate(app):
            path = app / 'meeting.ics'
            text = path.read_text().replace('UID:synthetic-design-1', 'UID:another-valid-uid')
            path.write_bytes(text.replace('\n', '\r\n').encode())
        self.verify(child, child['gold'], 0, mutate)


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
