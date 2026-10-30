"""Only the approved two APT URI edits may enter the pre-agent image."""
import asyncio
import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import AsyncMock, patch

from suite import variants
from suite.state import atomic_json, digest, load, tree

PARENT = 'rstan-to-pystan'
ID = 'rstan-to-pystan-compat-https-apt'
BASE = 'alexgshaw/rstan-to-pystan@sha256:b23d48839408524326095c15ada4145c772b87646cf299022ff1f58c97baba91'
REAL = Path(__file__).resolve().parents[1] / 'cases' / PARENT / 'variant'


class HTTPSAptVariantTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.cases = Path(self.temp.name) / 'cases'; self.root = self.cases / PARENT / 'variant'
        self.root.mkdir(parents=True)
        for name in ('Dockerfile', 'base-packages.json', 'ubuntu.sources.before', 'ubuntu.sources.after'):
            (self.root / name).write_bytes((REAL / name).read_bytes())
        self.contract = {'variant_id': ID, 'original_parent_id': PARENT,
            'counts_as_original_official': False, 'dependency_phase': 'image_build_before_agent',
            'base_image': BASE, 'image': 'vw-rstan-https-apt@sha256:' + 'a' * 64,
            'dockerfile_sha256': digest(self.root / 'Dockerfile'),
            'base_packages_sha256': digest(self.root / 'base-packages.json'),
            'sources_before_sha256': digest(self.root / 'ubuntu.sources.before'),
            'sources_after_sha256': digest(self.root / 'ubuntu.sources.after')}
        atomic_json(self.root / 'contract.json', self.contract)
        scope = patch.object(variants, 'CASES', self.cases); scope.start(); self.addCleanup(scope.stop)

    def descriptor(self):
        return variants.resolve(PARENT, {'id': ID, 'contract_sha256': digest(self.root / 'contract.json')})

    def test_authorized_descriptor_materializes_only_identity_and_image_changes(self):
        descriptor = self.descriptor()
        self.assertFalse(descriptor['counts_as_original_official'])
        self.assertEqual(variants.parent_for(descriptor), PARENT)
        official = Path(__file__).resolve().parents[3] / 'promptfoo/results/qwen38-suite/source/tasks' / PARENT
        before = tree(official)
        package = variants.materialize(official, Path(self.temp.name) / 'task', descriptor)
        self.assertEqual(tree(package), variants.execution_files(official, descriptor))
        self.assertEqual(tree(official), before)
        self.assertEqual({n for n in before if before[n] != tree(package)[n]}, {'task.toml'})
        with self.assertRaises(ValueError): variants.resolve('mcmc-sampling-stan', descriptor)

    def test_rehashed_recipe_cannot_install_or_inject_environment(self):
        self.descriptor()
        original = (self.root / 'Dockerfile').read_text()
        for extra in ('RUN apt-get update\n', 'COPY answer /app/answer\n', 'ENV PIP_INDEX_URL=other\n'):
            with self.subTest(extra=extra):
                (self.root / 'Dockerfile').write_text(original + extra)
                atomic_json(self.root / 'contract.json', {**self.contract,
                    'dockerfile_sha256': digest(self.root / 'Dockerfile')})
                with self.assertRaises(ValueError): self.descriptor()

    def test_rehashed_sources_inventory_base_and_official_flag_are_rejected(self):
        self.descriptor()
        for name, key in [('ubuntu.sources.before', 'sources_before_sha256'),
                          ('ubuntu.sources.after', 'sources_after_sha256'),
                          ('base-packages.json', 'base_packages_sha256')]:
            original = (self.root / name).read_bytes()
            (self.root / name).write_bytes(original + b'\n')
            atomic_json(self.root / 'contract.json', {**self.contract, key: digest(self.root / name)})
            with self.assertRaises(ValueError): self.descriptor()
            (self.root / name).write_bytes(original)
        for key, value in [('base_image', 'other'), ('counts_as_original_official', True),
                           ('dependency_phase', 'verifier_only')]:
            atomic_json(self.root / 'contract.json', {**self.contract, key: value})
            with self.assertRaises(ValueError): self.descriptor()

    def test_pre_agent_probe_reads_exact_sources_and_unchanged_package_inventory(self):
        descriptor = self.descriptor(); inventory = load(self.root / 'base-packages.json')
        raw = ''.join(f'{n}\t{v["version"]}\t{v["architecture"]}\n' for n, v in inventory.items()).encode()
        for sources, rows, valid in [((self.root / 'ubuntu.sources.after').read_bytes(), raw, True),
                                    ((self.root / 'ubuntu.sources.before').read_bytes(), raw, False),
                                    ((self.root / 'ubuntu.sources.after').read_bytes(), raw + b'extra\t1\tall\n', False)]:
            stream = io.StringIO()
            with patch('subprocess.check_output', return_value=rows) as call, \
                 patch('pathlib.Path.read_bytes', return_value=sources), contextlib.redirect_stdout(stream):
                exec(variants.probe_code(descriptor), {})
            self.assertEqual(call.call_args.args[0][0], 'dpkg-query')
            packet = {'environment_variant': descriptor, 'phase': 'before_agent',
                      'facts': json.loads(stream.getvalue())}
            if valid: variants.check_start(packet, descriptor)
            else:
                with self.assertRaises(ValueError): variants.check_start(packet, descriptor)

    def test_standard_backend_requires_matching_pre_agent_proof(self):
        from harbor.environments.base import ExecResult
        from harbor.environments.docker.docker import DockerEnvironment
        from harbor.models.task.config import EnvironmentConfig
        from harbor.models.trial.paths import TrialPaths
        from suite.docker import SuiteDockerEnvironment
        descriptor = self.descriptor(); root = Path(self.temp.name); envdir = root / 'env'; envdir.mkdir()
        paths = TrialPaths(trial_dir=root / 'trial'); paths.mkdir()
        env = SuiteDockerEnvironment(environment_dir=envdir, environment_name='https', session_id='unit',
            trial_paths=paths, task_env_config=EnvironmentConfig(cpus=4, memory_mb=8192,
            docker_image=descriptor['image']), environment_variant=descriptor)
        facts = variants.expected_facts(descriptor)
        with patch.object(DockerEnvironment, 'start', new=AsyncMock()), \
             patch.object(env, 'exec', new=AsyncMock(return_value=ExecResult(
                 stdout=json.dumps(facts), stderr=None, return_code=0))):
            asyncio.run(env.start())
        variants.check_start(load(paths.trial_dir / 'environment-variant.json'), descriptor)
        env.task_env_config.docker_image = 'other'
        with patch.object(DockerEnvironment, 'start', new=AsyncMock()) as start:
            with self.assertRaises(ValueError): asyncio.run(env.start())
            start.assert_not_awaited()

    def test_record_schema_keeps_variant_out_of_official_and_other_parents(self):
        from datetime import datetime, timezone
        from suite.scoring import experiment_record
        from suite.contract import validate_record
        descriptor = self.descriptor()
        task = {'taskId': PARENT, 'kind': 'compatibility_variant', 'environment_variant': descriptor, 'files': {}}
        row = experiment_record({'experiment_id': 'synthetic', 'adapter': {'sha256': 'a'*64},
             'runtime': {'binarySha256': 'b'*64}}, task,
             {'attempt_id': 'one', 'started_at': datetime.now(timezone.utc).isoformat()}, {}, 'c'*64)
        validate_record(row)
        for bad in ({**row, 'kind': 'official'}, {**row, 'case_id': 'mcmc-sampling-stan'}):
            with self.assertRaises(ValueError): validate_record(bad)
