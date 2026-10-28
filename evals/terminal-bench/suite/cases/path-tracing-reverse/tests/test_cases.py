"""Execute compiled C submissions, including independent semantic mutations."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from suite.diagnostics import copy_files, load_case

ROOT = Path(__file__).resolve().parents[1]
CASE, _ = load_case(ROOT / 'case.json')


def execute(child, files, override=None):
    with tempfile.TemporaryDirectory() as directory:
        app, oracle = Path(directory)/'app', Path(directory)/'oracle'
        app.mkdir()
        oracle.mkdir()
        copy_files(ROOT, child['fixtures'], app)
        copy_files(ROOT, files, app)
        copy_files(ROOT, child['oracle'], oracle)
        if override is not None:
            (app/'reproduce.c').write_text(override)
        return subprocess.run([sys.executable, str(ROOT/'check.py'), str(app), str(oracle)],
                              capture_output=True, timeout=40)


class Cases(unittest.TestCase):
    def test_constant_replay_rejected(self):
        for child in CASE['children']:
            with self.subTest(child=child['id']):
                result = execute(child, {}, '#include <stdio.h>\nint main(){puts("0 0");}\n')
                self.assertEqual(result.returncode, 1, result.stderr.decode())

    def test_invalid_source_rejected(self):
        result = execute(CASE['children'][0], {}, 'this is not C\n')
        self.assertEqual(result.returncode, 1, result.stderr.decode())

    def test_reference_dependency_rejected(self):
        result = execute(CASE['children'][0], {},
                         '#include <stdlib.h>\nint main(){return system("python3 probe.py 0 0");}\n')
        self.assertEqual(result.returncode, 1, result.stderr.decode())


def add(child, name, files, expected):
    def test(self):
        result = execute(child, files)
        self.assertEqual(result.returncode, expected, result.stderr.decode())
    setattr(Cases, 'test_'+child['id']+'_'+name.replace('-', '_'), test)


for child in CASE['children']:
    add(child, 'gold', child['gold'], 0)
    add(child, 'nop', {}, 1)
    for negative in child['negatives']:
        add(child, negative['id'], negative['files'], 1)
