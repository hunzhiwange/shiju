"""Independent real-process controls; no model output or self-authored tests."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from suite.diagnostics import CHECKER_BOOTSTRAP, copy_files, load_case

ROOT = Path(__file__).resolve().parents[1]


class Cases(unittest.TestCase):
    def test_checker_imports_standard_library_from_candidate_working_directory(self):
        case, _ = load_case(ROOT / 'case.json')
        child = next(child for child in case['children'] if child['id'] == 'm2')
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory) / 'app'
            oracle = Path(directory) / 'oracle'
            app.mkdir()
            oracle.mkdir()
            copy_files(ROOT, child['gold'], app)
            copy_files(ROOT, child['oracle'], oracle)
            # select is built in on macOS; json also shadows stdlib on the host.
            (app / 'json.py').write_text('raise RuntimeError("candidate import")\n')
            bootstrap = CHECKER_BOOTSTRAP.replace('/tests/check.py', str(ROOT / 'check.py'))
            bootstrap = bootstrap.replace("'/app'", repr(str(app)))
            bootstrap = bootstrap.replace("'/tests/oracle'", repr(str(oracle)))
            result = subprocess.run([sys.executable, '-c', bootstrap], cwd=app,
                                    capture_output=True, timeout=60)
            self.assertEqual(result.returncode, 0, result.stderr.decode())

    def test_gold_nop_and_three_semantic_negatives_per_child(self):
        case, _ = load_case(ROOT / 'case.json')
        count = 0
        for child in case['children']:
            candidates = [('gold', child['gold'], 0), ('nop', {}, 1)]
            candidates += [(n['id'], n['files'], 1) for n in child['negatives']]
            for name, files, expected in candidates:
                with self.subTest(child=child['id'], artifact=name):
                    with tempfile.TemporaryDirectory() as directory:
                        app = Path(directory) / 'app'
                        oracle = Path(directory) / 'oracle'
                        app.mkdir()
                        oracle.mkdir()
                        copy_files(ROOT, child['fixtures'], app)
                        copy_files(ROOT, files, app)
                        copy_files(ROOT, child['oracle'], oracle)
                        result = subprocess.run(
                            [sys.executable, str(ROOT / 'check.py'), str(app), str(oracle)],
                            capture_output=True, timeout=60)
                        self.assertEqual(result.returncode, expected, result.stderr.decode())
                        count += 1
        self.assertEqual(count, 20)

    def test_delivery_accepts_synonymous_codons_and_linker_boundaries(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location('protein_checker', ROOT / 'check.py')
        checker = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(checker)
        domains = ['DKEAM', 'VAKEL', 'FKDAE', 'EAKVD', 'KAFDE']
        coding = dict(zip('DKEAMVLF', 'GAT AAG GAG GCC ATG GTG CTG TTC'.split()))
        coding.update(G='GGT', S='TCT')
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory)
            for linker in ('GSGSG', 'GSGSGSGSGSGSGSGSGSGS'):
                peptide = linker.join(domains)
                dna = ''.join(coding[a] for a in peptide)
                (app / 'gblock.txt').write_text(dna)
                self.assertTrue(checker.delivery(app))

    def test_delivery_rejects_format_frame_missing_domain_and_terminal_linker(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location('protein_checker', ROOT / 'check.py')
        checker = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(checker)
        dna = (ROOT / 'oracle/m4/gold/gblock.txt').read_text().strip()
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory)
            for bad in ('>sequence\n'+dna, dna+'\n\n', dna+'A', dna[:-15],
                        'GGT'*5+dna, dna+'TAA', dna.lower(), 'ACGT'*751):
                with self.subTest(artifact=bad[:12], length=len(bad)):
                    (app / 'gblock.txt').write_text(bad)
                    self.assertFalse(checker.delivery(app))
