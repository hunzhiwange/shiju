"""Exercise actual candidate files against the evaluator in a separate process."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from suite.diagnostics import copy_files, load_case

ROOT = Path(__file__).resolve().parents[1]


class Cases(unittest.TestCase):
    def run_checker(self, child, files=None, text=None):
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory) / 'app'
            oracle = Path(directory) / 'oracle'
            app.mkdir()
            oracle.mkdir()
            copy_files(ROOT, child['fixtures'], app)
            copy_files(ROOT, files or {}, app)
            copy_files(ROOT, child['oracle'], oracle)
            if text is not None:
                (app / 'result.json').write_text(text)
            return subprocess.run(
                [sys.executable, str(ROOT / 'check.py'), str(app), str(oracle)],
                capture_output=True, timeout=10)

    def test_gold_and_all_negative_artifacts(self):
        case, _ = load_case(ROOT / 'case.json')
        for child in case['children']:
            candidates = [('gold', child['gold'], 0), ('nop', {}, 1)]
            candidates += [(n['id'], n['files'], 1) for n in child['negatives']]
            for name, files, expected in candidates:
                with self.subTest(child=child['id'], artifact=name):
                    actual = self.run_checker(child, files)
                    self.assertEqual(actual.returncode, expected, actual.stderr.decode())

    def test_malformed_and_wrongly_typed_artifacts_are_rejected(self):
        case, _ = load_case(ROOT / 'case.json')
        task = json.loads((ROOT / 'oracle/m1/task.json').read_text())
        floating = json.loads(json.dumps(task['expected']))
        floating[0]['balance_cents'] = float(floating[0]['balance_cents'])
        controls = ['null', '{', json.dumps(floating),
                    '[{"id":"K019","id":"K019","name":"Ada Finch","balance_cents":7501}]']
        for text in controls:
            with self.subTest(text=text):
                actual = self.run_checker(case['children'][0], text=text)
                self.assertEqual(actual.returncode, 1, actual.stderr.decode())

    def test_nonfunctional_program_and_comparison_mutation_are_rejected(self):
        case, _ = load_case(ROOT / 'case.json')
        programs = [(1, 'calculate.py', 'print("{}")\n'),
                    (2, 'update.py', 'raise SystemExit(3)\n'),
                    (3, 'compare.py', 'import pathlib,sys\n'
                     'pathlib.Path(sys.argv[1],"ACCOUNTS.DAT").write_text("changed")\n'
                     'print(\'{"equal":true,"differences":[]}\')\n')]
        for index, name, program in programs:
            with self.subTest(child=index + 1):
                with tempfile.TemporaryDirectory() as directory:
                    source = Path(directory) / name
                    source.write_text(program)
                    # copy_files rejects external sources; stage this control directly.
                    app = Path(directory) / 'app'
                    oracle = Path(directory) / 'oracle'
                    app.mkdir()
                    oracle.mkdir()
                    (app / name).write_text(program)
                    copy_files(ROOT, case['children'][index]['oracle'], oracle)
                    result = subprocess.run([sys.executable, str(ROOT / 'check.py'),
                                             str(app), str(oracle)], capture_output=True, timeout=10)
                    self.assertEqual(result.returncode, 1, result.stderr.decode())
    def test_calculation_inputs_obey_the_declared_numeric_domain(self):
        from decimal import Decimal, ROUND_HALF_UP
        task = json.loads((ROOT / 'oracle/m2/task.json').read_text())
        for value in task['inputs']:
            with self.subTest(value=value):
                gross, discount, rate = (value[k] for k in
                                         ('gross_cents', 'discount_cents', 'tax_basis_points'))
                self.assertTrue(0 <= discount <= gross < 10**12)
                self.assertTrue(0 <= rate <= 10000)
                tax = (Decimal(gross - discount) * Decimal(rate) / Decimal(10000)).quantize(
                    Decimal(1), rounding=ROUND_HALF_UP)
                self.assertLess(gross - discount + int(tax), 10**12)
