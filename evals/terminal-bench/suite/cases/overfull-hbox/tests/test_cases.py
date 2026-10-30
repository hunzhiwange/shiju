"""Real TeX integration is skipped, never passed, when the engine is missing."""
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
CASE = json.loads((ROOT/'case.json').read_text())
spec = importlib.util.spec_from_file_location('overfull_checker', ROOT/'check.py')
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class SourceConstraints(unittest.TestCase):
    def test_allowed_family_member(self):
        self.assertTrue(checker.allowed_change('large, big\n', 'large!', 'big!'))

    def test_no_deletion(self):
        self.assertFalse(checker.allowed_change('large, big\n', 'large!', '!'))

    def test_no_punctuation_edit(self):
        self.assertFalse(checker.allowed_change('large, big\n', 'large!', 'big?'))

    def test_preserve_spaces(self):
        self.assertFalse(checker.allowed_change('large, big\n', 'large  cat', 'big cat'))

    def test_protect_command(self):
        self.assertFalse(checker.allowed_change('large, big\n', r'\large large', r'\big big'))

    def test_unlisted_word(self):
        self.assertFalse(checker.allowed_change('large, big\n', 'cat large', 'dog big'))

    def test_synonym_not_substring(self):
        self.assertFalse(checker.allowed_change('large, big\n', 'largely', 'bigly'))

    def test_missing_engine_is_infrastructure(self):
        with patch.object(checker.shutil, 'which', return_value=None):
            with self.assertRaises(checker.InfrastructureError):
                checker.compile_document({'main': 'x', 'input': ''})


class TransformPrograms(unittest.TestCase):
    def test_gold_programs_preserve_commands_and_replace_words(self):
        for child in CASE['children'][1:3]:
            request = {'input': r'\noindent large  cat!',
                       'synonyms': 'large, big\nnoindent, xx\n'}
            program = ROOT/child['gold']['answer.py']
            result = subprocess.run([sys.executable, str(program)],
                                    input=json.dumps(request), capture_output=True,
                                    text=True, timeout=5)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout), {'input': r'\noindent big  cat!'})

    def test_protected_command_mutants_are_rejected(self):
        for child in CASE['children'][1:3]:
            request = {'input': r'\noindent large  cat!',
                       'synonyms': 'large, big\nnoindent, xx\n'}
            mutant = next(n for n in child['negatives'] if n['id'] == 'changes-protected')
            program = ROOT/mutant['files']['answer.py']
            result = subprocess.run([sys.executable, str(program)],
                                    input=json.dumps(request), capture_output=True,
                                    text=True, timeout=5)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(checker.allowed_change(request['synonyms'], request['input'],
                                                   json.loads(result.stdout)['input']))


class TexArtifacts(unittest.TestCase):
    pass


def add_test(child, identity, mapping, expected):
    @unittest.skipUnless(shutil.which('pdflatex'), 'real pdflatex missing: calibration pending')
    def test(self):
        with tempfile.TemporaryDirectory() as directory:
            app, oracle = Path(directory)/'app', Path(directory)/'oracle'
            for files, dest in [(child['fixtures'], app), (mapping, app), (child['oracle'], oracle)]:
                dest.mkdir(parents=True, exist_ok=True)
                for target, source in files.items():
                    output = dest/target
                    output.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(ROOT/source, output)
            result = subprocess.run([sys.executable, str(ROOT/'check.py'), str(app), str(oracle)], capture_output=True, text=True, timeout=110)
            self.assertEqual(result.returncode, expected, result.stderr)
    setattr(TexArtifacts, f"test_{child['id']}_{identity.replace('-', '_')}", test)


for child in CASE['children']:
    for identity, files, expected in [('gold', child['gold'], 0), ('nop', {}, 1)] + [(n['id'], n['files'], 1) for n in child['negatives']]:
        add_test(child, identity, files, expected)
