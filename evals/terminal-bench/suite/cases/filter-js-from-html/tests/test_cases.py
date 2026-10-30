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


    def test_oracle_preserves_entity_text_and_safe_url(self):
        import json
        spec=json.loads((ROOT/'oracle/m3/expected.json').read_text())
        events=spec['outputs']['page.html']
        self.assertIn(['text', 'javascript: is plain text; 中文 & café'],events)
        self.assertIn(['start','a',[['href','https://example.test/javascript:notes']]],events)
        self.assertNotIn('script',[e[1] for e in events if e[0]=='start'])

    def test_replaced_html_symlink_is_rejected(self):
        child = CASE['children'][3]
        def mutate(app):
            p = app / 'filter.py'
            p.write_text(p.read_text() + "\nfrom pathlib import Path\np=Path(__import__('sys').argv[1]);b=p.read_bytes();p.unlink();q=p.with_suffix('.saved');q.write_bytes(b);p.symlink_to(q)\n")
        self.verify(child, child['gold'], 1, mutate)


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
