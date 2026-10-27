"""Independent candidate-file checks: no model, self-report, or parent answer."""
import json
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
            app, oracle = Path(directory) / 'app', Path(directory) / 'oracle'
            app.mkdir()
            oracle.mkdir()
            copy_files(ROOT, child['fixtures'], app)
            copy_files(ROOT, files, app)
            copy_files(ROOT, child['oracle'], oracle)
            if mutate:
                mutate(app)
            result = subprocess.run([sys.executable, str(ROOT / 'check.py'), str(app), str(oracle)],
                                    capture_output=True, timeout=10)
            self.assertEqual(result.returncode, expected, result.stderr.decode())

    def test_fixture_contracts_match_gold(self):
        def gold(child):
            path = ROOT / next(iter(child['gold'].values()))
            return json.loads(path.read_text()) if path.suffix == '.json' else path.read_text()
        # m1 is visually reviewed against frame.png; independent explicit transcription.
        self.assertEqual(gold(CASE['children'][0]),
                         ['examine brass dial', 'turn dial 7', 'say "Silver?"'])
        frames = json.loads((ROOT / 'fixtures/m2/snapshots.json').read_text())
        submitted = {}
        for frame in frames:
            if frame['submitted']:
                submitted.setdefault(frame['prompt_id'], frame['input'])
        self.assertEqual(gold(CASE['children'][1]), list(submitted.values()))
        frames = json.loads((ROOT / 'fixtures/m3/frames.json').read_text())
        rows = {}
        for frame in frames:
            for row in frame['rows']:
                rows.setdefault(row['id'], {})[row['part']] = row['text']
        expected = [' '.join(parts[k] for k in sorted(parts)).removeprefix('> ')
                    for _, parts in sorted(rows.items())]
        self.assertEqual(gold(CASE['children'][2]), expected)
        events = json.loads((ROOT / 'fixtures/m4/events.json').read_text())
        expected = ''.join(e['command'] + '\n' for e in sorted(
            events, key=lambda e: (e['time_ms'], e['sequence'])))
        self.assertEqual(gold(CASE['children'][3]), expected)

    def test_input_tampering_is_rejected(self):
        for child in CASE['children']:
            self.verify(child, child['gold'], 1, lambda app:
                        (app / next(iter(child['fixtures']))).write_bytes(b'changed'))

    def test_output_symlinks_are_rejected(self):
        for child in CASE['children']:
            output = next(iter(child['gold']))
            def mutate(app):
                (app / output).rename(app / 'saved')
                (app / output).symlink_to(app / 'saved')
            self.verify(child, child['gold'], 1, mutate)

    def test_malformed_and_non_array_json_are_rejected(self):
        for child in CASE['children'][:3]:
            for invalid in ('null', '{}', '[true]', '["look",', '[1]'):
                self.verify(child, child['gold'], 1, lambda app:
                            (app / 'result.json').write_text(invalid))

    def test_json_whitespace_is_accepted(self):
        for child in CASE['children'][:3]:
            self.verify(child, child['gold'], 0, lambda app:
                        (app / 'result.json').write_text(json.dumps(
                            json.loads((app / 'result.json').read_text()), indent=None)))

    def test_text_formatting_errors_are_rejected(self):
        child = CASE['children'][3]
        text = (ROOT / next(iter(child['gold'].values()))).read_text()
        for invalid in (text.rstrip('\n'), text.replace('\n', '\r\n'),
                        text + '\n', text.replace('look\n', '', 1),
                        '1. ' + text, text.upper()):
            self.verify(child, child['gold'], 1, lambda app:
                        (app / 'solution.txt').write_bytes(invalid.encode()))


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
