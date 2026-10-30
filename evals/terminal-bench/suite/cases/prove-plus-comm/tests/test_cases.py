"""Separate host contract tests and real Coq calibration; no mocked kernel."""
import importlib.util
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from suite.diagnostics import copy_files, load_case, run_checker

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('proof_checker', ROOT / 'check.py')
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class Cases(unittest.TestCase):
    def test_all_declared_assets_and_distinct_goals(self):
        case, hashes = load_case(ROOT / 'case.json')
        self.assertEqual(len(case['children']), 4)
        goals = {(ROOT / child['oracle']['task.json']).read_text()
                 for child in case['children']}
        self.assertEqual(len(goals), 4)
        self.assertGreater(len(hashes), 20)

    def test_forbidden_escape_and_unproved_premises_rejected(self):
        for body in ('admit.', 'Admitted.', 'sorry.',
                     'Axiom x : False.', 'Parameter x : False.',
                     'exact (False_rect _ x).', 'Qed. Theorem x : True.',
                     'ltac:(idtac).', '(* comment *) reflexivity.',
                     'Require Import Evil.', 'Set Guard Checking.',
                     'Unset Guard Checking.', 'native_compute.',
                     'abstract admit.', 'exact "Closed under the global context".'):
            with self.subTest(body=body):
                self.assertFalse(checker.permitted(body))

    def test_gold_is_in_closed_tactic_language(self):
        case, _ = load_case(ROOT / 'case.json')
        for child in case['children']:
            self.assertTrue(checker.permitted((ROOT / child['gold']['proof.txt']).read_text()))

    def test_missing_or_fabricated_artifact_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory)
            (app / 'Submission.vo').write_text('fake kernel success')
            self.assertFalse(checker.check(app, ROOT / 'oracle/m1'))
            (app / 'outside').write_text('reflexivity.')
            (app / 'proof.txt').symlink_to(app / 'outside')
            self.assertFalse(checker.check(app, ROOT / 'oracle/m1'))

    def test_real_kernel_gold_nop_and_three_negatives_per_child(self):
        case, _ = load_case(ROOT / 'case.json')
        count = 0
        for child in case['children']:
            candidates = [('gold', child['gold'], 0), ('nop', {}, 1)]
            candidates += [(n['id'], n['files'], 1) for n in child['negatives']]
            for name, files, expected in candidates:
                with self.subTest(child=child['id'], candidate=name):
                    if not (shutil.which('coqc') and shutil.which('coqchk')):
                        actual = run_checker(case['image'], ROOT, child, files,
                                             parent_id=case['parent_id'])
                        self.assertEqual(actual, expected)
                        count += 1
                        continue
                    with tempfile.TemporaryDirectory() as directory:
                        app, oracle = Path(directory)/'app', Path(directory)/'oracle'
                        app.mkdir(); oracle.mkdir()
                        copy_files(ROOT, child['fixtures'], app)
                        copy_files(ROOT, files, app)
                        copy_files(ROOT, child['oracle'], oracle)
                        actual = subprocess.run([sys.executable, str(ROOT/'check.py'),
                                                 str(app), str(oracle)],
                                                capture_output=True, timeout=75)
                        self.assertEqual(actual.returncode, expected, actual.stderr.decode())
                        count += 1
        self.assertEqual(count, 20)
