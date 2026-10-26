"""Filesystem acceptance tests; only the Harbor execution boundary is replaced."""
import asyncio
import importlib
import json
from pathlib import Path
import sys
import tempfile
import types
import unittest

# The default Python need not have Harbor. Check the real ABC separately with
# the pinned Harbor interpreter; this substitute models only its boundary.
try:
    from harbor.agents.installed.base import BaseInstalledAgent
except ImportError:
    boundary = types.ModuleType('harbor.agents.installed.base')
    class BaseInstalledAgent:
        def __init__(self, logs_dir, model_name=None, **kwargs):
            self.logs_dir, self.model_name = Path(logs_dir), model_name
        async def exec_as_agent(self, environment, **kwargs):
            return await environment.exec(**kwargs)
    boundary.BaseInstalledAgent = BaseInstalledAgent
    boundary.NonZeroAgentExitCodeError = type("NonZeroAgentExitCodeError", (RuntimeError,), {})
    sys.modules['harbor.agents.installed.base'] = boundary


def stream(*events):
    return ''.join(json.dumps({'id': i, 'event': event}) + '\n'
                   for i, event in enumerate(events)) + '{"type":"stream_end","status":"terminated"}\n'


class Environment:
    def __init__(self, output=None, code=0, healthy=True, interruption=None):
        self.output = output or stream({'type': 'chat.done', 'finish_reason': 'stop',
                                      'usage': {'input_tokens': 3, 'output_tokens': 2}})
        self.code, self.healthy, self.interruption = code, healthy, interruption
        self.calls, self.uploads = [], {}
    async def upload_file(self, source_path, target_path):
        self.uploads[target_path] = Path(source_path).read_bytes()
    async def download_file(self, source_path, target_path):
        name = source_path.rsplit('/', 1)[-1]
        if name == 'events.jsonl':
            Path(target_path).write_text(self.output)
        else:
            Path(target_path).write_text('')
    async def exec(self, command, **kwargs):
        self.calls.append((command, kwargs))
        if 'chat-stream ' in command:
            if self.interruption:
                raise self.interruption
            return types.SimpleNamespace(return_code=self.code, stdout=self.output, stderr='')
        if '/v1/health' in command:
            return types.SimpleNamespace(return_code=0 if self.healthy else 1, stdout='', stderr='')
        return types.SimpleNamespace(return_code=0, stdout='', stderr='')


class AdapterTests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.logs = Path(self.temp.name)
        self.module = importlib.import_module('vibewindow_agent')
        self.agent = self.module.VibeWindowAgent(logs_dir=self.logs, task_id='fix-git')
        self.context = types.SimpleNamespace(metadata=None)
    def result(self):
        paths = list(self.logs.glob('*/result.json'))
        self.assertEqual(len(paths), 1)
        return json.loads(paths[0].read_text())
    async def test_profile_binding_uses_only_frozen_provider_and_cleans_payloads(self):
        profile = self.logs / 'profile.json'
        profile.write_text(json.dumps({'provider': {'api': 'https://example.test/v1',
            'adapter': 'openai-compatible', 'models': {'deepseek-v4-flash-0731': {}}},
            'auth': {'type': 'api', 'key': 'fixture-secret'}}))
        self.agent = self.module.VibeWindowAgent(logs_dir=self.logs, task_id='fix-git',
                                               profile_path=str(profile))
        env = Environment()
        await self.agent.run('task', env, self.context)
        uploaded = {p.rsplit('/', 1)[-1]: v for p, v in env.uploads.items()}
        self.assertEqual(json.loads(uploaded['auth.json'])['key'], 'fixture-secret')
        self.assertEqual(list(json.loads(uploaded['provider.json'])['providers']), ['token-plan'])
        self.assertTrue(any('bind-profile.sh' in c[0] for c in env.calls))
        self.assertFalse(any('fixture-secret' in c[0] for c in env.calls))
        self.assertTrue(any('rm -rf' in c[0] and self.agent.remote in c[0] for c in env.calls))
        self.assertTrue(self.result()['profile_bound'])

    async def test_profile_rejects_extra_models_before_upload(self):
        profile = self.logs / 'bad-profile.json'
        profile.write_text(json.dumps({'provider': {'api': 'https://example.test/v1',
            'models': {'other': {}}}, 'auth': {'type': 'api', 'key': 'fixture-secret'}}))
        self.agent = self.module.VibeWindowAgent(logs_dir=self.logs, task_id='fix-git',
                                               profile_path=str(profile))
        env = Environment()
        with self.assertRaises(ValueError):
            await self.agent.run('task', env, self.context)
        self.assertFalse(any(p.endswith('/auth.json') for p in env.uploads))
        self.assertTrue(any('rm -rf' in c[0] for c in env.calls))

    async def test_failed_agent_maps_to_harbor_scored_failure(self):
        from harbor.agents.installed.base import NonZeroAgentExitCodeError
        with self.assertRaises(NonZeroAgentExitCodeError):
            await self.agent.run('task', Environment(code=7), self.context)
        self.assertEqual(self.result()['exit_code'], 7)
        self.assertEqual(self.result()['status'], 'failed')

    async def test_explicit_runtime_manifest_preserves_frozen_task_and_model(self):
        manifest = json.loads(Path(self.module.__file__).with_name('manifest.json').read_text())
        manifest['runtime']['binarySha256'] = '0' * 64
        path = self.logs / 'runtime-manifest.json'
        path.write_text(json.dumps(manifest))
        agent = self.module.VibeWindowAgent(logs_dir=self.logs, task_id='fix-git',
                                           runtime_manifest=str(path))
        self.assertEqual(agent.manifest['runtime']['binarySha256'], '0' * 64)
        self.assertEqual(agent.model_name, 'token-plan/deepseek-v4-flash-0731')
        self.assertEqual(agent.task['agentTimeoutSec'], 900)
        self.assertNotEqual(self.agent.manifest['runtime']['binarySha256'], '0' * 64)

    async def test_runtime_manifest_rejects_different_model(self):
        manifest = json.loads(Path(self.module.__file__).with_name('manifest.json').read_text())
        manifest['model']['id'] = 'other/model'
        path = self.logs / 'runtime-manifest.json'
        path.write_text(json.dumps(manifest))
        with self.assertRaises(ValueError):
            self.module.VibeWindowAgent(logs_dir=self.logs, task_id='fix-git',
                                       runtime_manifest=str(path))

    async def test_parser_uses_jsonl_file_without_exec_diagnostics(self):
        class NoisyEnvironment(Environment):
            async def exec(self, command, **kwargs):
                result = await super().exec(command, **kwargs)
                if 'chat-stream ' in command:
                    result.stdout = 'runtime diagnostic on exec output'
                return result
        await self.agent.run('task', NoisyEnvironment(), self.context)
        self.assertEqual(self.result()['status'], 'completed')

    async def test_optional_done_reason_uses_last_finished_step(self):
        output = stream({'type': 'chat.step_start', 'step_index': 1},
                        {'type': 'chat.step_finish', 'step_index': 1, 'finish_reason': 'stop'},
                        {'type': 'chat.done', 'finish_reason': None})
        await self.agent.run('task', Environment(output=output), self.context)
        self.assertEqual(self.result()['status'], 'completed')
        self.assertIsNone(self.result()['terminal']['finish_reason'])
        self.assertEqual(self.result()['finish_reason_source'], 'last_step_finish')

    async def test_optional_done_reason_does_not_hide_length_limit(self):
        output = stream({'type': 'chat.step_finish', 'finish_reason': 'length'},
                        {'type': 'chat.done', 'finish_reason': None})
        with self.assertRaises(RuntimeError):
            await self.agent.run('task', Environment(output=output), self.context)
        self.assertEqual(self.result()['status'], 'failed')

    async def test_optional_done_reason_rejects_unfinished_new_step(self):
        output = stream({'type': 'chat.step_finish', 'finish_reason': 'stop'},
                        {'type': 'chat.step_start', 'step_index': 2},
                        {'type': 'chat.done', 'finish_reason': None})
        with self.assertRaises(RuntimeError):
            await self.agent.run('task', Environment(output=output), self.context)
        self.assertEqual(self.result()['status'], 'failed')

    async def test_reported_zero_usage_with_model_activity_is_missing(self):
        zero = {'input_tokens': 0, 'output_tokens': 0, 'cached_tokens': 0}
        output = stream({'type': 'chat.delta', 'delta': 'answer'},
                        {'type': 'chat.done', 'finish_reason': 'stop', 'usage': zero})
        await self.agent.run('task', Environment(output=output), self.context)
        self.assertTrue(self.result()['usage_missing'])
        self.assertIsNone(self.result()['usage'])
        self.assertEqual(self.result()['reported_usage'], zero)

    async def test_install_requires_verified_runtime(self):
        with self.assertRaises(ValueError):
            await self.agent.install(Environment())
        with self.assertRaises(ValueError):
            self.module.VibeWindowAgent(logs_dir=self.logs, task_id='unknown')
        with self.assertRaises(ValueError):
            self.module.VibeWindowAgent(logs_dir=self.logs, task_id='fix-git', model_name='other')

    async def test_instruction_is_transferred_without_shell_interpolation(self):
        env = Environment()
        instruction = 'Find logs\n$(touch /tmp/unsafe) 中文'
        await self.agent.run(instruction, env, self.context)
        body = json.loads(next(iter(env.uploads.values())))
        self.assertEqual(body['messages'], [{'role': 'user', 'content': instruction}])
        self.assertEqual(body['model'], 'token-plan/deepseek-v4-flash-0731')
        self.assertEqual(self.result()['usage']['input_tokens'], 3)
        self.assertNotIn('reward', self.result())
        self.assertEqual(self.result()['status'], 'completed')
        self.assertTrue(list(self.logs.glob('*/events.jsonl')))
    async def test_adapter_preserves_failure_and_task_directory(self):
        env = Environment(code=7)
        with self.assertRaises(RuntimeError):
            await self.agent.run('task', env, self.context)
        result = self.result()
        self.assertEqual(result['exit_code'], 7)
        self.assertEqual(result['directory'], '/app/personal-site')
        command, options = next(c for c in env.calls if 'chat-stream ' in c[0])
        self.assertIn('--directory /app/personal-site', command)
        self.assertEqual(options['cwd'], '/app/personal-site')
        self.assertEqual(options['timeout_sec'], 900)
        self.assertEqual(result['status'], 'failed')
    async def test_request_failure_and_missing_terminal_are_not_completion(self):
        for output in (stream({'type': 'chat.error', 'error': 'HTTP 503'}),
                       stream({'type': 'chat.delta', 'delta': 'I succeeded'}),
                       'not-json\n'):
            with self.subTest(output=output):
                with self.assertRaises(RuntimeError):
                    await self.agent.run('task', Environment(output=output), self.context)
        for path in self.logs.glob('*/result.json'):
            result = json.loads(path.read_text())
            self.assertEqual(result['status'], 'failed')
            self.assertTrue(result['usage_missing'])
    async def test_gateway_not_ready_is_archived_and_stopped(self):
        env = Environment(healthy=False)
        with self.assertRaises(RuntimeError):
            await self.agent.run('task', env, self.context)
        self.assertEqual(self.result()['status'], 'failed')
        self.assertFalse(any('chat-stream ' in c[0] for c in env.calls))
        self.assertTrue(any('kill' in c[0] for c in env.calls))
    async def test_cancellation_and_timeout_are_archived_and_stopped(self):
        for interruption, status in ((asyncio.CancelledError(), 'cancelled'),
                                     (TimeoutError(), 'timed_out')):
            with self.subTest(status=status):
                env = Environment(interruption=interruption)
                with self.assertRaises(type(interruption)):
                    await self.agent.run('task', env, self.context)
                result = json.loads(sorted(self.logs.glob('*/result.json'),
                                           key=lambda p: p.stat().st_mtime_ns)[-1].read_text())
                self.assertEqual(result['status'], status)
                self.assertTrue(any(p.read_text() == env.output
                                    for p in self.logs.glob('*/events.jsonl')))
                self.assertTrue(any('chat-stop ' in c[0] for c in env.calls))
                self.assertTrue(any('kill' in c[0] for c in env.calls))

if __name__ == '__main__':
    unittest.main()
