"""Offline suite binding and inherited Gateway transport acceptance tests."""
import asyncio
import copy
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import threading
from types import SimpleNamespace
import unittest
import urllib.request

os.environ['LITELLM_LOCAL_MODEL_COST_MAP'] = 'True'

from harbor.agents.installed.base import NonZeroAgentExitCodeError
from suite.agent import SuiteAgent, source_identity

MODEL = 'token-plan/qwen3.8-flash'
LIMIT = {'context': 200000, 'input': 200000, 'output': 32000}


class Boundary(BaseHTTPRequestHandler):
    calls = []
    reason = 'stop'

    def log_message(self, *args):
        pass

    def do_PATCH(self):
        self.receive()

    def do_PUT(self):
        self.receive()

    def receive(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        self.server.calls.append((self.command, self.path, body))
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b'{}')

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        self.server.calls.append(('POST', self.path, body))
        self.send_response(200)
        self.send_header('Content-Type', 'text/event-stream')
        self.end_headers()
        event = {'type': 'chat.done', 'finish_reason': self.server.reason}
        if self.server.reason == 'error':
            event = {'type': 'chat.error', 'finish_reason': 'error', 'retryable': False}
        self.wfile.write(('data: ' + json.dumps(event) + '\n\n').encode())


class Environment:
    """Fake installed runtime; real loopback HTTP is the uncertain boundary."""
    def __init__(self, server):
        self.server = server
        self.files = {}
        self.modes = {}
        self.commands = []

    async def upload_file(self, source, target):
        self.files[str(target)] = Path(source).read_bytes()
        self.modes[str(target)] = Path(source).stat().st_mode & 0o777

    async def download_file(self, source, target):
        if source not in self.files:
            raise FileNotFoundError(source)
        Path(target).write_bytes(self.files[source])

    async def exec(self, *, command, **kwargs):
        self.commands.append(command)
        if command.startswith('bash ') and '/bind-profile.sh ' in command:
            # Run the actual immutable bootstrap, with only its filesystem mocked.
            with tempfile.TemporaryDirectory() as temp:
                remote = shlex.split(command)[-1]
                for name in ('provider.json', 'auth.json', 'bind-profile.sh'):
                    path = Path(temp) / name
                    path.write_bytes(self.files[remote + '/' + name])
                    path.chmod(0o600)
                proc = subprocess.run(['bash', str(Path(temp) / 'bind-profile.sh'), temp],
                                      capture_output=True, text=True)
                return SimpleNamespace(return_code=proc.returncode, stderr=proc.stderr)
        if 'chat-stream --request-id' in command:
            request_path = command.split(' --input ')[1].split()[0]
            body = self.files[request_path]
            request = urllib.request.Request('http://127.0.0.1:42619/chat-stream',
                                             data=body, headers={'Content-Type': 'application/json'})
            with urllib.request.urlopen(request) as response:
                event = json.loads(response.read().decode().removeprefix('data: ').strip())
            records = [{'event': event}, {'type': 'stream_end', 'status': 'terminated'}]
            self.files[request_path.rsplit('/', 1)[0] + '/events.jsonl'] = (
                '\n'.join(json.dumps(record) for record in records)).encode()
            return SimpleNamespace(return_code=0 if self.server.reason == 'stop' else 1, stderr='')
        return SimpleNamespace(return_code=0, stderr='')


class AgentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = ThreadingHTTPServer(('127.0.0.1', 42619), Boundary)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join()

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.manifest = {'model': {'id': MODEL, 'max_output_tokens': 32000,
                                   'input_window_tokens': 200000},
                         'tasks': [{'taskId': 'fixture', 'workingDirectory': '/app',
                                    'agentTimeoutSec': 30}],
                         'runtime': {'binarySha256': 'a' * 64,
                                     'requiredSidecars': {'libsqlite3.so': 'b' * 64}}}
        self.profile = {'provider': {'api': 'http://127.0.0.1:42619',
                                     'adapter': 'openai-compatible',
                                     'models': {'qwen3.8-flash': {'status': 'active', 'limit': LIMIT}}},
                        'auth': {'type': 'api', 'key': 'offline-fixture-key'}}
        self.server.calls = []
        self.server.reason = 'stop'

    def agent(self, **kwargs):
        manifest = self.root / 'manifest.json'
        manifest.write_text(json.dumps(self.manifest))
        profile = self.root / 'profile.json'
        profile.write_text(json.dumps(self.profile))
        profile.chmod(0o600)
        return SuiteAgent(self.root / 'logs', task_id='fixture',
                          runtime_manifest=manifest, profile_path=profile, **kwargs)

    def test_binding_sends_exact_model_and_budget_with_private_files(self):
        agent = self.agent()
        environment = Environment(self.server)
        asyncio.run(agent._bind_profile(environment, '/fixture'))
        self.assertEqual(self.server.calls, [
            ('PATCH', '/v1/config', {'providers': {'token-plan': self.profile['provider']}, 'reliability': {'provider_retries': 0}}),
            ('PUT', '/v1/auth/token-plan', self.profile['auth'])])
        for name in ('provider.json', 'auth.json'):
            self.assertEqual(environment.modes['/fixture/' + name], 0o600)
        self.assertNotIn('offline-fixture-key', ' '.join(environment.commands))
        self.assertEqual(agent.model_name, MODEL)

    def test_stop_length_and_nonretryable_have_one_request_and_raw_evidence(self):
        for reason in ('stop', 'length', 'error'):
            with self.subTest(reason=reason):
                self.server.calls = []
                self.server.reason = reason
                agent = self.agent()
                context = SimpleNamespace(metadata={})
                environment = Environment(self.server)
                if reason == 'stop':
                    asyncio.run(agent.run('fixture instruction', environment, context))
                else:
                    with self.assertRaises(NonZeroAgentExitCodeError):
                        asyncio.run(agent.run('fixture instruction', environment, context))
                requests = [body for method, _, body in self.server.calls if method == 'POST']
                self.assertEqual(len(requests), 1)
                self.assertEqual(requests[0]['model'], MODEL)
                result = context.metadata['vibewindow']
                self.assertEqual(result['finish_reason'], reason)
                self.assertEqual(result['exit_code'], 0 if reason == 'stop' else 1)
                self.assertEqual(result['status'], 'completed' if reason == 'stop' else 'failed')
                self.assertEqual(result['suite_source_identity'], source_identity())
                self.assertEqual(result['adapter_source_sha256'], source_identity()['sha256'])
                self.assertEqual(result['base_adapter_source_sha256'],
                                 source_identity()['files']['vibewindow_agent.py'])
                self.assertEqual(result['output_truncated'], reason == 'length')
                self.assertNotIn('reward', result)
                archived = Path(agent.logs_dir) / result['request_id'] / 'result.json'
                self.assertEqual(json.loads(archived.read_text()), result)
                if reason == 'error':
                    self.assertIs(result['terminal']['retryable'], False)

    def test_rejects_model_and_budget_drift(self):
        original = copy.deepcopy(self.manifest)
        for field, value in (('id', 'other/model'), ('max_output_tokens', 32001),
                             ('input_window_tokens', 200001), ('max_output_tokens', 32000.0)):
            self.manifest = copy.deepcopy(original)
            self.manifest['model'][field] = value
            with self.assertRaises(ValueError):
                self.agent()
        self.manifest = original
        with self.assertRaises(ValueError):
            self.agent(model_name='other/model')

    def test_rejects_profile_drift_before_http(self):
        for mutation in ('model', 'budget', 'float_budget', 'auth', 'disabled', 'extra'):
            with self.subTest(mutation=mutation):
                profile = copy.deepcopy(self.profile)
                if mutation == 'model':
                    profile['provider']['models']['other'] = {}
                elif mutation == 'budget':
                    profile['provider']['models']['qwen3.8-flash']['limit']['output'] = 4096
                elif mutation == 'float_budget':
                    profile['provider']['models']['qwen3.8-flash']['limit']['output'] = 32000.0
                elif mutation == 'auth':
                    profile['auth']['type'] = 'oauth'
                elif mutation == 'disabled':
                    profile['provider']['models']['qwen3.8-flash']['status'] = 'disabled'
                else:
                    profile['provider']['headers'] = {'Authorization': 'secret'}
                path = self.root / 'invalid.json'
                path.write_text(json.dumps(profile))
                path.chmod(0o600)
                agent = self.agent()
                agent.profile_path = path
                with self.assertRaises(ValueError):
                    asyncio.run(agent._bind_profile(Environment(self.server), '/fixture'))
                self.assertEqual(self.server.calls, [])

    def test_rejects_missing_profile_and_ambiguous_task(self):
        with self.assertRaises(ValueError):
            SuiteAgent(self.root / 'logs', task_id='fixture', runtime_manifest='unused')
        self.manifest['tasks'] *= 2
        with self.assertRaises(ValueError):
            self.agent()

    def test_rejects_second_attempt_without_http(self):
        agent = self.agent()
        context = SimpleNamespace(metadata={})
        environment = Environment(self.server)
        asyncio.run(agent.run('fixture', environment, context))
        calls = list(self.server.calls)
        with self.assertRaisesRegex(ValueError, 'retry forbidden'):
            asyncio.run(agent.run('fixture', environment, context))
        self.assertEqual(self.server.calls, calls)

    def test_rejects_unsafe_task_runtime_and_public_profile(self):
        original = copy.deepcopy(self.manifest)
        for timeout in (0, -1, float('inf'), True):
            self.manifest = copy.deepcopy(original)
            self.manifest['tasks'][0]['agentTimeoutSec'] = timeout
            with self.assertRaises(ValueError):
                self.agent()
        self.manifest = copy.deepcopy(original)
        self.manifest['runtime']['requiredSidecars']['../escape'] = 'a' * 64
        with self.assertRaises(ValueError):
            self.agent()
        self.manifest = original
        agent = self.agent()
        agent.profile_path.chmod(0o644)
        with self.assertRaises(ValueError):
            asyncio.run(agent._bind_profile(Environment(self.server), '/fixture'))
        self.assertEqual(self.server.calls, [])

    def test_identity_covers_wrapper_base_and_binding_script(self):
        identity = source_identity()
        self.assertEqual(set(identity['files']), {'suite/agent.py', 'vibewindow_agent.py', 'bind-profile.sh', 'suite/budget.py', 'suite/budget_gate.rs'})
        self.assertEqual(len(identity['sha256']), 64)

    def test_diagnostic_requires_frozen_gate_and_installation_before_binding(self):
        self.manifest['tasks'][0]['kind'] = 'diagnostic'
        with self.assertRaisesRegex(ValueError, 'native budget gate'):
            self.agent()
        self.manifest['runtime']['requiredSidecars']['vw-budget-gate'] = 'c' * 64
        agent = self.agent()
        self.assertIn('before_llm_call', agent.runtime_env['VW_REQUIRED_EXECUTION_HOOKS'])
        with self.assertRaisesRegex(ValueError, 'installed before'):
            asyncio.run(agent._bind_profile(Environment(self.server), '/fixture'))
        self.assertEqual(self.server.calls, [])


class StreamFactsTests(unittest.TestCase):
    def test_error_before_done_is_preserved_independently_of_last_terminal(self):
        raw = '\n'.join(json.dumps(row) for row in [
            {'event': {'type': 'chat.error'}},
            {'event': {'type': 'chat.done', 'finish_reason': 'stop'}},
            {'type': 'stream_end', 'status': 'terminated'}])
        result = SuiteAgent.summarize_stream(raw, 0)
        self.assertTrue(result['error_seen'])
        self.assertTrue(result['stream_terminated'])
        self.assertEqual(result['status'], 'failed')

    def test_missing_stream_end_is_not_inferred_from_terminal(self):
        result = SuiteAgent.summarize_stream(json.dumps(
            {'event': {'type': 'chat.done', 'finish_reason': 'stop'}}), 0)
        self.assertFalse(result['stream_terminated'])
        self.assertEqual(result['status'], 'failed')
