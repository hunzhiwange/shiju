"""Execute synthetic recovery programs; never invoke Docker or paid models."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from suite.diagnostics import copy_files, load_case, materialize

ROOT = Path(__file__).resolve().parents[1]

class Cases(unittest.TestCase):
    def verify(self, child, files, expected):
        with tempfile.TemporaryDirectory() as directory:
            app, oracle = Path(directory) / 'app', Path(directory) / 'oracle'
            app.mkdir(); oracle.mkdir()
            copy_files(ROOT, child['fixtures'], app)
            copy_files(ROOT, child['oracle'], oracle)

            for target, source in files.items():
                import shutil
                shutil.copyfile(ROOT / source, app / target)
            result = subprocess.run([sys.executable, str(ROOT / 'check.py'), str(app), str(oracle)], capture_output=True, timeout=15)
            self.assertEqual(result.returncode, expected, result.stderr.decode())

    def test_precomputed_answers_rejected(self):
        case, _ = load_case(ROOT / 'case.json')
        for child in case['children']:
            expected = json.loads((ROOT / child['oracle']['task.json']).read_text())['expected'][0]
            with self.subTest(child=child['id']), tempfile.TemporaryDirectory() as directory:
                program = Path(directory) / 'solution.py'
                program.write_text('import pathlib,sys\npathlib.Path(sys.argv[2]).write_text(' + repr(json.dumps(expected)) + ')\n')
                self.verify(child, {'solution.py':str(program)}, 1)

    def test_no_hidden_material_in_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tasks'
            materialize(ROOT / 'case.json', output)
            for task in output.iterdir():
                files = {str(p.relative_to(task / 'environment')) for p in (task / 'environment').rglob('*') if p.is_file()}
                self.assertEqual(files, {'Dockerfile', 'fixtures/trunc.db'})

def add_control(child, name, files, expected):
    def test(self):
        self.verify(child, files, expected)
    setattr(Cases, 'test_' + child['id'] + '_' + name.replace('-', '_'), test)

for child in load_case(ROOT / 'case.json')[0]['children']:
    for name, files, expected in [('gold', child['gold'], 0), ('nop', {}, 1)] + [(n['id'], n['files'], 1) for n in child['negatives']]:
        add_control(child, name, files, expected)

if __name__ == '__main__':
    unittest.main()
