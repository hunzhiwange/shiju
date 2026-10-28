"""Host TDD of actual OpenSSL artifacts; Docker calibration remains separate."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
CASE = json.loads((ROOT / 'case.json').read_text())


class CertificateCases(unittest.TestCase):
    pass


def add_test(child, identity, files, expected):
    def test(self):
        with tempfile.TemporaryDirectory(prefix='openssl-case-') as directory:
            work = Path(directory)
            app, oracle = work/'app', work/'oracle'
            app.mkdir(); oracle.mkdir()
            for mapping, destination in [(child['fixtures'], app), (files, app), (child['oracle'], oracle)]:
                for target, source in mapping.items():
                    path = destination/target
                    path.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(ROOT/source, path)
            process = subprocess.run([sys.executable, str(ROOT/'check.py'), str(app), str(oracle)],
                                     capture_output=True, text=True, timeout=90)
            self.assertEqual(process.returncode, expected, process.stderr)
    setattr(CertificateCases, f"test_{child['id']}_{identity.replace('-', '_')}", test)


for child in CASE['children']:
    for identity, files, expected in [('gold', child['gold'], 0), ('nop', {}, 1)] + [
            (item['id'], item['files'], 1) for item in child['negatives']]:
        add_test(child, identity, files, expected)

if __name__ == '__main__':
    unittest.main()
