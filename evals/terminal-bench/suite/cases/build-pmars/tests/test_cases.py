"""Exercise real diagnostic programs in fresh resource-limited containers."""
from pathlib import Path
import unittest
from suite import diagnostics

ROOT = Path(__file__).resolve().parents[1]


class CaseTests(unittest.TestCase):
    def test_gold_nop_and_three_mutations_per_child(self):
        case, _ = diagnostics.load_case(ROOT / 'case.json')
        for child in case['children']:
            candidates = [('gold', child['gold'], 0), ('nop', {}, 1)]
            candidates += [(n['id'], n['files'], 1) for n in child['negatives']]
            for name, files, expected in candidates:
                with self.subTest(child=child['id'], candidate=name):
                    self.assertEqual(diagnostics.run_checker(case['image'], ROOT, child, files), expected)

    def test_replaced_source_archive_is_rejected(self):
        import hashlib
        import importlib.util
        import json
        import tarfile
        import tempfile
        spec = importlib.util.spec_from_file_location('pmars_checker', ROOT / 'check.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        case = json.loads((ROOT / 'case.json').read_text())
        child = case['children'][0]
        with tempfile.TemporaryDirectory() as tmp:
            app, oracle = Path(tmp) / 'app', Path(tmp) / 'oracle'
            for mapping, target in [(child['fixtures'], app), (child['gold'], app),
                                    (child['oracle'], oracle)]:
                diagnostics.copy_files(ROOT, mapping, target)
            archive = app / 'pmars_1.4.7.orig.tar.gz'
            with tarfile.open(archive, 'w:gz'):
                pass
            sha = hashlib.sha256(archive.read_bytes()).hexdigest()
            (app / 'pmars.dsc').write_text('Checksums-Sha256:\n '
                + sha + ' ' + str(archive.stat().st_size) + ' ' + archive.name + '\n')
            report = json.loads((app / 'source.json').read_text())
            report['archive_sha256'] = sha
            (app / 'source.json').write_text(json.dumps(report))
            self.assertFalse(module.check(app, oracle))
