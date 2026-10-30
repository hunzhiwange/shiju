"""Approved native variant preserves original task and refuses identity drift."""
import asyncio
import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import AsyncMock, patch

from suite import variants
from suite.state import digest, load, tree

PARENT = 'crack-7z-hash'
ID = 'crack-7z-hash-arm64-compat'


class CrackVariantTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.cases = Path(self.temp.name) / 'cases'
        self.root = self.cases / PARENT / 'variant'
        self.root.mkdir(parents=True)
        source = Path(__file__).resolve().parents[1] / 'cases' / PARENT / 'variant'
        for name in ('packages.lock', 'deb-lock.json', 'SHA256SUMS'):
            shutil.copyfile(source / name, self.root / name)
        (self.root / 'Dockerfile').write_text('FROM --platform=linux/arm64 ubuntu@sha256:24737cef17f4e22b1977df7d9a05e77c4e28b7da5b8911f8d8ccda136bf3573c\n')
        self.contract = {
            'schema_version': 1, 'variant_id': ID, 'original_parent_id': PARENT,
            'counts_as_original_official': False, 'dependency_phase': 'image_build_before_agent',
            'image': 'vw-crack-7z-arm64-compat@sha256:' + 'a' * 64,
            'architecture': 'aarch64', 'john_commit': '8b5bfefbdc9b643513037f30c8200f5dc0ffc841',
            'john_tree': 'ee4163c03a0f0792d2f3fb928751bc6a889e6029',
            'source_tar_sha256': '08082dd15e6396f256598b452049a8ce996dc7d4478710cb8cb71645fb0b9a36',
            'john_binary_sha256': 'b' * 64, 'package_files_sha256': 'c' * 64,
            'packages_lock_sha256': digest(self.root / 'packages.lock'),
            'deb_lock_sha256': digest(self.root / 'deb-lock.json'),
            'sha256sums_sha256': digest(self.root / 'SHA256SUMS'),
            'dockerfile_sha256': digest(self.root / 'Dockerfile'),
        }
        self.save()
        self.patcher = patch.object(variants, 'CASES', self.cases)
        self.patcher.start()
        self.addCleanup(self.patcher.stop)

    def save(self):
        (self.root / 'contract.json').write_text(json.dumps(self.contract) + '\n')

    def descriptor(self):
        return variants.resolve(PARENT, {'id': ID, 'contract_sha256': digest(self.root / 'contract.json')})

    def test_resolves_only_approved_source_architecture_and_complete_package_closure(self):
        value = self.descriptor()
        self.assertFalse(value['counts_as_original_official'])
        for name, changed in [('architecture', 'x86_64'), ('john_commit', '0' * 40),
                              ('source_tar_sha256', '0' * 64), ('counts_as_original_official', True)]:
            original = self.contract[name]
            self.contract[name] = changed
            self.save()
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.descriptor()
            self.contract[name] = original
        self.save()
        original = (self.root / 'packages.lock').read_text()
        (self.root / 'packages.lock').write_text(original + 'extra-package=1\n')
        self.contract['packages_lock_sha256'] = digest(self.root / 'packages.lock')
        self.save()
        with self.assertRaises(ValueError):
            self.descriptor()

    def test_materialization_changes_only_name_and_image_and_rejects_input_or_oracle_drift(self):
        descriptor = self.descriptor()
        official = Path(__file__).resolve().parents[3] / 'promptfoo/results/qwen38-suite/source/tasks' / PARENT
        before = tree(official)
        package = variants.materialize(official, Path(self.temp.name) / PARENT, descriptor)
        self.assertEqual(tree(package), variants.execution_files(official, descriptor))
        self.assertEqual(tree(official), before)
        for name in ['instruction.md', 'solution/solve.sh', 'tests/test_outputs.py']:
            path = package / name
            original = path.read_bytes()
            path.write_bytes(original + b'\n# altered\n')
            with self.subTest(name=name), self.assertRaises(ValueError):
                variants.check_package(package, descriptor)
            path.write_bytes(original)

    def test_pre_agent_proof_rejects_package_binary_source_or_architecture_drift(self):
        descriptor = self.descriptor()
        facts = variants.expected_facts(descriptor)
        receipt = {'environment_variant': descriptor, 'phase': 'before_agent', 'facts': facts}
        variants.check_start(receipt, descriptor)
        for name in facts:
            changed = copy.deepcopy(receipt)
            changed['facts'][name] = 'changed'
            with self.subTest(name=name), self.assertRaises(ValueError):
                variants.check_start(changed, descriptor)
        from suite import crack_variant
        command = crack_variant.probe_command(self.root, self.contract)
        self.assertNotIn('apt-get', command)
        self.assertNotIn('python3', command)
        self.assertIn('dpkg-query', command)
        self.assertIn('sha256sum', command)

    def test_shared_backend_routes_shell_probe_and_records_proof_without_python_dependency(self):
        from harbor.environments.base import ExecResult
        from harbor.environments.docker.docker import DockerEnvironment
        from harbor.models.task.config import EnvironmentConfig
        from harbor.models.trial.paths import TrialPaths
        from suite.docker import SuiteDockerEnvironment
        descriptor = self.descriptor()
        envdir = Path(self.temp.name) / 'env'
        envdir.mkdir()
        paths = TrialPaths(trial_dir=Path(self.temp.name) / 'trial')
        paths.mkdir()
        env = SuiteDockerEnvironment(environment_dir=envdir, environment_name='crack', session_id='synthetic',
            trial_paths=paths, task_env_config=EnvironmentConfig(cpus=1, memory_mb=4096, docker_image=descriptor['image']),
            environment_variant=descriptor)
        facts = variants.expected_facts(descriptor)
        with patch.object(DockerEnvironment, 'start', new=AsyncMock()), patch.object(env, 'exec',
                new=AsyncMock(return_value=ExecResult(stdout=json.dumps(facts), stderr=None, return_code=0))) as execute:
            asyncio.run(env.start())
            self.assertNotIn('python3', execute.call_args.kwargs['command'])
        variants.check_start(load(paths.trial_dir / 'environment-variant.json'), descriptor)

    def test_result_schema_and_report_keep_variant_separate_from_official(self):
        from suite.scoring import experiment_record
        from suite.contract import validate_record
        from suite.report import build_report
        from suite.tests.test_report import CATALOG
        from datetime import datetime, timezone
        descriptor = self.descriptor()
        task = {'taskId': PARENT, 'kind': 'compatibility_variant', 'environment_variant': descriptor, 'files': {}}
        row = experiment_record({'experiment_id': 'synthetic', 'adapter': {'sha256': 'a' * 64},
            'runtime': {'binarySha256': 'b' * 64}}, task,
            {'attempt_id': 'one', 'started_at': datetime.now(timezone.utc).isoformat()}, {}, 'c' * 64)
        validate_record(row)
        for changed in ({**row, 'kind': 'official'}, {**row, 'case_id': 'build-cython-ext'}):
            with self.assertRaises(ValueError):
                validate_record(changed)
        report = build_report(CATALOG, [row])
        self.assertEqual(report['layers']['official']['observed'], 0)
        self.assertEqual(report['layers']['compatibility_variant']['observed'], 1)

    def test_scheduler_identity_covers_native_variant_enforcement(self):
        from suite import state
        before = state.execution_identity()
        original = state.digest
        with patch.object(state, 'digest', side_effect=lambda p: '0' * 64
                          if Path(p).name == 'crack_variant.py' else original(p)):
            self.assertNotEqual(state.execution_identity(), before)
