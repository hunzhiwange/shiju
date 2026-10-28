"""Production and preparation must enter a create-time resource-limited backend."""
import asyncio
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from suite import scoring


class EntryTests(unittest.TestCase):
    def test_shared_harbor_entry_selects_limited_backend_before_trial_creation(self):
        captured = []
        class Result:
            def model_dump(self, **kwargs):
                return {}
        class Trial:
            async def run(self):
                return Result()
        async def create(config):
            captured.append(config)
            return Trial()
        with patch('harbor.trial.trial.Trial.create', side_effect=create):
            asyncio.run(scoring.harbor_trial(Path('/fixture'), Path('/trials'), 'parent', 'nop'))
            asyncio.run(scoring.harbor_trial(Path('/fixture-m1'), Path('/trials'), 'child', 'oracle'))
            asyncio.run(scoring.harbor_trial(Path('/fixture'), Path('/trials'), 'paid-entry',
                                            'suite.agent:SuiteAgent', kwargs={}))
        self.assertEqual([c.environment.import_path for c in captured],
                         ['suite.docker:SuiteDockerEnvironment'] * 3)


import json
from types import SimpleNamespace
from unittest.mock import AsyncMock
from harbor.environments.base import ExecResult
from harbor.environments.docker.docker import DockerEnvironment
from harbor.models.task.config import EnvironmentConfig
from harbor.models.trial.paths import TrialPaths
from suite.docker import SuiteDockerEnvironment, build_command, active_resources, limits


class BuildTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        resource_lock = patch('suite.docker.LOCK', self.root / 'resources.lock')
        resource_lock.start()
        self.addCleanup(resource_lock.stop)
        environment = self.root / 'environment'
        environment.mkdir()
        (environment / 'Dockerfile').write_text('FROM cached\nRUN true\n')
        self.paths = TrialPaths(trial_dir=self.root / 'trial')
        self.paths.mkdir()
        self.env = SuiteDockerEnvironment(environment_dir=environment, environment_name='test',
            session_id='resource-test', trial_paths=self.paths,
            task_env_config=EnvironmentConfig(cpus=1, memory_mb=512))
        self.service = {'image': 'same-harbor-tag', 'platform': 'linux/amd64',
                        'build': {'context': str(environment), 'network': 'none'}}

    def test_every_run_is_bounded_before_build_and_context_network_platform_are_preserved(self):
        command = build_command(self.service, 1, 512 * 2**20)
        for flag, value in [('--memory', str(512 * 2**20)), ('--memory-swap', str(512 * 2**20)),
                            ('--cpu-period', '100000'), ('--cpu-quota', '100000'),
                            ('--network', 'none'), ('--platform', 'linux/amd64'), ('-t', 'same-harbor-tag')]:
            self.assertEqual(command[command.index(flag) + 1], value)
        self.assertEqual(command[-1], self.service['build']['context'])
        del self.service['build']['network']
        self.assertNotIn('--network', build_command(self.service, 1, 512 * 2**20))

    def test_unsupported_build_or_invalid_budget_has_no_unlimited_fallback(self):
        self.service['build']['secrets'] = ['secret']
        with self.assertRaisesRegex(ValueError, 'unlimited fallback'):
            build_command(self.service, 1, 512 * 2**20)
        for cpu, memory in [(0, 512), (float('nan'), 512), (1, 0), (5, 512), (1, 9000 * 2**20)]:
            with self.subTest(cpu=cpu, memory=memory), self.assertRaises(ValueError):
                limits(cpu, memory)

    def test_compose_build_and_up_share_declared_service_limits(self):
        commands = []
        services = {'main': {**self.service, 'cpus': 1, 'mem_limit': 512 * 2**20},
                    'harbor-docker-egress-control-sidecar': {'image': 'cached-sidecar'}}
        async def parent(env, command, *args, **kwargs):
            commands.append(command)
            return ExecResult(stdout=json.dumps({'services': services}), stderr=None, return_code=0)
        with patch.object(DockerEnvironment, '_run_docker_compose_command', new=parent), \
             patch.object(self.env, '_limited_build', new=AsyncMock()) as build, \
             patch.object(self.env, '_admit', new=AsyncMock()) as admit:
            asyncio.run(self.env._run_docker_compose_command(['build']))
            build.assert_awaited_once_with(self.service | {'cpus': 1, 'mem_limit': 512 * 2**20}, 1.0, 512 * 2**20)
            asyncio.run(self.env._run_docker_compose_command(['up', '-d']))
            admit.assert_awaited_once_with(1.25, 576 * 2**20)
        self.assertIn(['up', '--no-build', '-d'], commands)
        self.assertNotIn(['build'], commands)
        overlay = json.loads((self.paths.trial_dir / 'suite-resources.json').read_text())
        self.assertEqual(overlay['services']['main']['mem_limit'], 512 * 2**20)
        self.assertEqual(overlay['services']['harbor-docker-egress-control-sidecar']['cpus'], .25)

    def test_admission_counts_classic_builder_quota_and_rejects_unbounded_active_work(self):
        container = {'Name': '/active-builder', 'HostConfig':
                     {'NanoCpus': 0, 'CpuQuota': 100000, 'CpuPeriod': 100000, 'Memory': 512 * 2**20}}
        self.assertEqual(active_resources([container]), (1, 512 * 2**20))
        container['HostConfig']['CpuQuota'] = 0
        with self.assertRaises(ValueError):
            active_resources([container])
        self.env._docker = AsyncMock(side_effect=['id', json.dumps([
            {'Name': '/other-trial', 'HostConfig': {'NanoCpus': 4 * 10**9, 'Memory': 512 * 2**20}}])])
        with self.assertRaisesRegex(ValueError, 'capacity'):
            asyncio.run(self.env._admit(1, 512 * 2**20))

    def test_buildkit_is_disabled_and_capacity_failure_prevents_process_creation(self):
        process = SimpleNamespace(returncode=0, communicate=AsyncMock(return_value=(b'cached', None)))
        with patch.object(self.env, '_admit', new=AsyncMock()), \
             patch('suite.docker.asyncio.create_subprocess_exec', new=AsyncMock(return_value=process)) as spawn:
            asyncio.run(self.env._limited_build(self.service, 1, 512 * 2**20))
        self.assertEqual(spawn.call_args.kwargs['env']['DOCKER_BUILDKIT'], '0')
        self.assertEqual(list(spawn.call_args.args), build_command(self.service, 1, 512 * 2**20))
        with patch.object(self.env, '_admit', new=AsyncMock(side_effect=ValueError('capacity'))), \
             patch('suite.docker.asyncio.create_subprocess_exec') as spawn:
            with self.assertRaises(ValueError):
                asyncio.run(self.env._limited_build(self.service, 1, 512 * 2**20))
            spawn.assert_not_called()

    def test_sidecar_uses_same_harbor_content_tag_and_limited_build_when_cache_is_absent(self):
        from contextlib import nullcontext
        from harbor.environments.docker.utils import docker_build_context_hash, _compute_image_name
        context = self.env._EGRESS_CONTROL_SIDECAR_CONTEXT_PATH
        dockerfile = self.env._egress_control_sidecar_dockerfile_path()
        expected = _compute_image_name(self.env._EGRESS_CONTROL_SIDECAR_DOCKER_NAME,
            docker_build_context_hash(context=context, dockerfile_path=dockerfile,
                                      build_args={}, platform='linux/amd64'))
        with patch('suite.docker.default_docker_platform', new=AsyncMock(return_value='linux/amd64')), \
             patch('suite.docker.docker_image_exists', new=AsyncMock(return_value=False)), \
             patch.object(self.env, '_resource_lock', return_value=nullcontext()), \
             patch.object(self.env, '_limited_build', new=AsyncMock()) as build:
            asyncio.run(self.env._ensure_egress_control_sidecar_image_built())
        self.assertEqual(self.env._env_vars.egress_control_sidecar_image_name, expected)
        service, cpu, memory = build.call_args.args
        self.assertEqual(service['image'], expected)
        self.assertEqual(service['build']['context'], str(context.resolve()))
        self.assertEqual(service['platform'], 'linux/amd64')
        self.assertEqual((cpu, memory), (1, 512 * 2**20))
        with patch('suite.docker.default_docker_platform', new=AsyncMock(return_value='linux/amd64')), \
             patch('suite.docker.docker_image_exists', new=AsyncMock(return_value=True)), \
             patch.object(self.env, '_resource_lock', return_value=nullcontext()), \
             patch.object(self.env, '_limited_build', new=AsyncMock()) as build:
            asyncio.run(self.env._ensure_egress_control_sidecar_image_built())
            build.assert_not_awaited()

    def test_implicit_compose_tag_is_preserved(self):
        service = {'build': self.service['build'], 'cpus': 1, 'mem_limit': 512 * 2**20}
        async def parent(*args, **kwargs):
            return ExecResult(stdout=json.dumps({'name': 'exact-project', 'services': {'main': service}}),
                              stderr=None, return_code=0)
        with patch.object(DockerEnvironment, '_run_docker_compose_command', new=parent):
            services, _ = asyncio.run(self.env._resolved_services())
        self.assertEqual(services['main']['image'], 'exact-project-main')

    def test_frozen_execution_identity_covers_build_strategy_bytes(self):
        from suite import state
        original = state.digest
        before = state.execution_identity()
        with patch.object(state, 'digest', side_effect=lambda p:
                          '0' * 64 if Path(p).name == 'docker.py' else original(p)):
            self.assertNotEqual(state.execution_identity(), before)

    def test_explicit_sidecar_budgets_are_preserved_and_unaccounted_replicas_rejected(self):
        services = {'main': {**self.service, 'cpus': 1, 'mem_limit': 512 * 2**20},
                    'harbor-docker-egress-control-sidecar': {'image': 'cached', 'cpus': .5,
                                                            'mem_limit': 128 * 2**20}}
        async def parent(*args, **kwargs):
            return ExecResult(stdout=json.dumps({'services': services}), stderr=None, return_code=0)
        with patch.object(DockerEnvironment, '_run_docker_compose_command', new=parent):
            _, overlay = asyncio.run(self.env._resolved_services())
            self.assertEqual(overlay['harbor-docker-egress-control-sidecar']['cpus'], .5)
            self.assertEqual(overlay['harbor-docker-egress-control-sidecar']['mem_limit'], 128 * 2**20)
            services['main']['deploy'] = {'replicas': 2}
            with self.assertRaisesRegex(ValueError, 'one replica'):
                asyncio.run(self.env._resolved_services())
