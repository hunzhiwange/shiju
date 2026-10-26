"""Synthetic artifacts and real isolated checker executions; no model calls."""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from suite.diagnostics import POLICY, materialize, validate, validate_all


class DiagnosticsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        # Preloaded Python 3 image; Docker --pull never forbids downloads.
        self.image = os.environ.get(
            'VW_DIAGNOSTIC_TEST_IMAGE',
            'vw-qwen-sqlite-verifier-deps@sha256:'
            '501ed2aec3dd40c3bc8ef631ee3fb55ccbce56b22cc7bed912e71b986b86dd25')
        self.case = {'schema_version': 1, 'parent_id': 'example',
                     'image': self.image, 'policy': copy.deepcopy(POLICY), 'children': []}
        for index in range(1, 5):
            prefix = f'm{index}'
            def write(name, content):
                path = self.root / prefix / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content)
                return path.relative_to(self.root).as_posix()
            instruction = write('instruction.md', 'Double input.txt and write result.txt.\n')
            checker = write('check.py',
                            'import pathlib,sys\n'
                            'a,b=map(pathlib.Path,sys.argv[1:])\n'
                            'assert not pathlib.Path("/solution").exists()\n'
                            'assert not pathlib.Path("/Users/Shared/work").exists()\n'
                            'try:\n'
                            ' ok=int((a/"result.txt").read_text())==int((b/"expected.txt").read_text())\n'
                            'except (OSError,ValueError):\n ok=False\n'
                            'sys.exit(0 if ok else 1)\n')
            self.case['children'].append({
                'id': prefix, 'instruction': instruction, 'checker': checker,
                'fixtures': {'input.txt': write('fixtures/input.txt', '2')},
                'oracle': {'expected.txt': write('oracle/expected.txt', '4')},
                'gold': {'result.txt': write('gold/result.txt', '4')},
                'negatives': [{'id': f'wrong-{n}', 'files': {
                    'result.txt': write(f'negative-{n}/result.txt', str(n))}}
                              for n in (0, 2, 8)]})
        self.path = self.root / 'case.json'
        self.save()

    def save(self):
        self.path.write_text(json.dumps(self.case))

    def reject(self):
        self.save()
        with self.assertRaises(ValueError):
            validate(self.path)

    def test_missing_fixture_rejected(self):
        (self.root / self.case['children'][0]['fixtures']['input.txt']).unlink()
        self.reject()

    def test_parent_solution_and_symlink_rejected(self):
        self.case['children'][0]['instruction'] = '../solution/solve.sh'
        self.reject()
        self.case['children'][0]['instruction'] = 'alias'
        (self.root / 'alias').symlink_to('/etc/hosts')
        self.reject()

    def test_duplicate_child_and_negative_ids_rejected(self):
        self.case['children'][1]['id'] = 'm1'
        self.reject()
        self.case['children'][1]['id'] = 'm2'
        self.case['children'][0]['negatives'][1]['id'] = 'wrong-0'
        self.reject()

    def test_budget_drift_and_unknown_fields_rejected(self):
        for key in POLICY:
            with self.subTest(key=key):
                old = self.case['policy'][key]
                self.case['policy'][key] = 'changed'
                self.reject()
                self.case['policy'][key] = old
        self.case['parent_solution'] = '/solution/solve.sh'
        self.reject()

    def test_hidden_material_cannot_be_public(self):
        child = self.case['children'][0]
        child['fixtures']['expected.txt'] = child['oracle']['expected.txt']
        self.reject()
        child['fixtures'].pop('expected.txt')
        (self.root / child['fixtures']['input.txt']).write_text('4')
        self.reject()

    def test_unpinned_image_and_unsafe_output_rejected(self):
        self.case['image'] = 'python:latest'
        self.reject()
        self.case['image'] = self.image
        self.case['children'][0]['gold']['../tests/answer'] = 'missing'
        self.reject()

    def test_materialization_is_standard_and_only_copies_allowlist(self):
        (self.root / 'secret').write_text('not public')
        out = self.root / 'packages'
        materialize(self.path, out)
        from harbor.models.task.config import TaskConfig
        for index in range(1, 5):
            task = out / f'example-m{index}'
            config = TaskConfig.model_validate(__import__('tomllib').loads(
                (task / 'task.toml').read_text()))
            self.assertEqual(config.agent.timeout_sec, 900)
            self.assertEqual(config.verifier.timeout_sec, 120)
            self.assertEqual(json.loads(config.metadata["policy_json"]), POLICY)
            files = {p.relative_to(task / 'environment').as_posix()
                     for p in (task / 'environment').rglob('*') if p.is_file()}
            self.assertEqual(files, {'Dockerfile', 'fixtures/input.txt'})
            self.assertTrue((task / 'tests/oracle/expected.txt').is_file())
            self.assertTrue((task / 'solution/solve.sh').is_file())
        # Execute emitted solution and verifier scripts in separate fresh containers.
        task = out / 'example-m1'
        app = self.root / 'rendered-app'
        logs = self.root / 'rendered-logs'
        app.mkdir()
        logs.mkdir()
        for phase, script in [('solution', '/solution/solve.sh'), ('tests', '/tests/test.sh')]:
            command = ['docker', 'run', '--pull', 'never', '--rm', '--network', 'none',
                       '--cpus', '1', '--memory', '512m', '--cap-drop', 'ALL',
                       '--mount', f'type=bind,src={app},dst=/app',
                       '--mount', f'type=bind,src={logs},dst=/logs',
                       '--mount', f'type=bind,src={task / phase},dst=/{phase},readonly',
                       '--entrypoint', 'sh', self.image, script]
            result = subprocess.run(command, capture_output=True, timeout=120)
            self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((logs / 'verifier/reward.txt').read_text().strip(), '1')
        with self.assertRaises(FileExistsError):
            materialize(self.path, out)

    def test_real_checker_gold_nop_mutations_and_receipt_tamper(self):
        receipt = validate(self.path, run_fixtures=True)
        self.assertEqual(len(receipt['checks']), 20)
        self.assertEqual([c['exit_code'] for c in receipt['checks']], [0, 1, 1, 1, 1] * 4)
        catalog = self.root / 'catalog.json'
        catalog.write_text(json.dumps({'tasks': [{'id': 'example'}]}))
        cases = self.root / 'cases'
        cases.mkdir()
        import shutil
        shutil.copytree(self.root / 'm1', cases / 'example/m1')
        for n in range(2, 5):
            shutil.copytree(self.root / f'm{n}', cases / f'example/m{n}')
        (cases / 'example/case.json').write_bytes(self.path.read_bytes())
        (cases / 'example/validation.json').write_text(json.dumps(receipt))
        validate_all(catalog, cases, verify_receipts=True)
        (cases / 'example/m1/negative-0/result.txt').write_text('4')
        with self.assertRaises(ValueError):
            validate_all(catalog, cases, verify_receipts=True)

    def test_cli_validation_and_coverage_fail_closed(self):
        result = subprocess.run(
            [__import__('sys').executable, '-m', 'suite.diagnostics',
             'validate', '--case', str(self.path)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)['parent_id'], 'example')
        catalog = self.root / 'catalog.json'
        catalog.write_text(json.dumps({'tasks': [{'id': 'missing'}]}))
        with self.assertRaises(ValueError):
            validate_all(catalog, self.root, verify_receipts=True)

    def test_checker_exception_on_negative_is_not_a_valid_rejection(self):
        child = self.case['children'][0]
        (self.root / child['checker']).write_text(
            'import pathlib,sys\n'
            'p=pathlib.Path(sys.argv[1])/"result.txt"\n'
            'if not p.exists(): sys.exit(1)\n'
            'if p.read_text()=="4": sys.exit(0)\n'
            'raise RuntimeError("broken negative path")\n')
        with self.assertRaises(ValueError):
            validate(self.path, run_fixtures=True)

    def test_invalid_negative_and_checker_crash_fail_calibration(self):
        child = self.case['children'][0]
        (self.root / child['negatives'][0]['files']['result.txt']).write_text('4')
        with self.assertRaises(ValueError):
            validate(self.path, run_fixtures=True)
        (self.root / child['checker']).write_text('raise RuntimeError("broken")\n')
        with self.assertRaises(ValueError):
            validate(self.path, run_fixtures=True)
