"""Single-task adapter for the pinned Harbor 0.24.0 interface.

Add this directory to PYTHONPATH and select vibewindow_agent:VibeWindowAgent.
Pass task_id and runtime_dir (task26 ELF plus the manifest sidecars). An explicit
container config_directory may bind a prepared profile; no host config is read.
Only instruction is sent to Gateway. Completion is transport state, never reward.
Raw logs belong in Harbor's trial logs/results, not in source control.
"""
import asyncio
import hashlib
import json
from pathlib import Path
import shlex
import tempfile
import uuid

from harbor.agents.installed.base import BaseInstalledAgent, NonZeroAgentExitCodeError


_ADAPTER_SOURCE_SHA256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()


class VibeWindowAgent(BaseInstalledAgent):
    def __init__(self, logs_dir, *, task_id, runtime_dir=None,
                 config_directory=None, profile_path=None, runtime_manifest=None, model_name=None, **kwargs):
        manifest = json.loads(Path(__file__).with_name('manifest.json').read_text())
        if runtime_manifest is not None:
            candidate = json.loads(Path(runtime_manifest).read_text())
            if candidate['model']['id'] != manifest['model']['id']:
                raise ValueError('runtime manifest differs from the frozen model')
            # Only runtime identity may change; task instruction/budgets stay frozen.
            manifest['runtime'] = candidate['runtime']
        tasks = [task for task in manifest['tasks'] if task['taskId'] == task_id]
        if len(tasks) != 1:
            raise ValueError('task_id must identify one frozen manifest task')
        model = manifest['model']['id']
        if model_name is not None and model_name != model:
            raise ValueError('model differs from the frozen manifest')
        super().__init__(logs_dir=Path(logs_dir), model_name=model, **kwargs)
        self.task = tasks[0]
        self.manifest = manifest
        self.runtime_dir = Path(runtime_dir) if runtime_dir else None
        self.profile_path = Path(profile_path) if profile_path else None
        self.remote = '/tmp/vw-harbor-' + uuid.uuid4().hex
        self.config_directory = config_directory or self.remote + '/config'
        self.runtime_env = {
            'HOME': self.remote + '/home', 'RUST_LOG': 'warn',
            'LD_LIBRARY_PATH': self.remote + '/runtime',
            'SSL_CERT_FILE': self.remote + '/runtime/ca-certificates.crt',
            'OPENSSL_CONF': self.remote + '/runtime/openssl.cnf',
        }

    @staticmethod
    def name():
        return 'vibewindow'

    def version(self):
        return 'task29-terminal-contract-harbor-0.24.0'

    async def _execute(self, environment, command, *, timeout_sec=15):
        # Use the environment with its configured default agent user.
        return await environment.exec(command=command, cwd=self.task['workingDirectory'],
                                      env=self.runtime_env, timeout_sec=timeout_sec)

    async def install(self, environment):
        if self.runtime_dir is None:
            raise ValueError('runtime_dir with task26 verified runtime files is required')
        expected = dict(self.manifest['runtime']['requiredSidecars'])
        expected['shiju'] = self.manifest['runtime']['binarySha256']
        expected['libsqlite3.so.0'] = expected['libsqlite3.so']
        for name, digest in expected.items():
            if hashlib.sha256((self.runtime_dir / name).read_bytes()).hexdigest() != digest:
                raise ValueError('runtime digest mismatch: ' + name)
        command = 'mkdir -p ' + ' '.join(shlex.quote(path) for path in (
            self.remote + '/runtime', self.remote + '/home', self.config_directory))
        result = await self._execute(environment, command)
        if result.return_code:
            raise RuntimeError('runtime directory creation failed')
        for name in expected:
            await environment.upload_file(self.runtime_dir / name, self.remote + '/runtime/' + name)
        result = await self._execute(environment, 'chmod 700 ' + self.remote + '/runtime/shiju')
        if result.return_code:
            raise RuntimeError('runtime installation failed')

    async def _bind_profile(self, environment, remote):
        if self.profile_path is None:
            return False
        profile = json.loads(self.profile_path.read_text())
        provider, auth = profile['provider'], profile['auth']
        if (set(provider) - {'api', 'adapter', 'models'}
                or set(provider.get('models', {})) != {'deepseek-v4-flash-0731'}
                or not provider.get('api') or set(auth) != {'type', 'key'}
                or auth.get('type') != 'api' or not auth.get('key')):
            raise ValueError('profile must contain only the frozen provider/model and API auth')
        with tempfile.TemporaryDirectory() as temp:
            for name, payload in (('provider.json', {'providers': {'token-plan': provider}}),
                                  ('auth.json', auth)):
                path = Path(temp) / name
                path.write_text(json.dumps(payload, ensure_ascii=True))
                path.chmod(0o600)
                await environment.upload_file(path, remote + '/' + name)
            await environment.upload_file(Path(__file__).with_name('bind-profile.sh'),
                                          remote + '/bind-profile.sh')
        bound = await self._execute(environment, 'bash ' + remote + '/bind-profile.sh ' + remote)
        if bound.return_code:
            raise RuntimeError('container profile binding failed')
        return True

    @staticmethod
    def summarize_stream(raw, exit_code):
        try:
            records = [json.loads(line) for line in raw.splitlines() if line.strip()]
            if any(not isinstance(record, dict) for record in records):
                raise ValueError('expected object records')
            events = [record['event'] for record in records if 'event' in record]
            if any(not isinstance(event, dict) for event in events):
                raise ValueError('expected object events')
        except ValueError as error:
            raise RuntimeError('invalid Gateway JSONL') from error
        terminal, reported_usage, last_step_reason = None, None, None
        activity = False
        for event in events:
            kind = event.get('type')
            if kind == 'chat.step_start':
                last_step_reason = None
            elif kind == 'chat.step_finish':
                last_step_reason = event.get('finish_reason')
            if event.get('usage') is not None:
                reported_usage = event['usage']
            if kind in ('chat.done', 'chat.error'):
                terminal = event
            if kind in ('chat.delta', 'chat.tool'):
                activity = True
        reason = (terminal or {}).get('finish_reason')
        reason_source = 'terminal'
        if reason is None:
            reason, reason_source = last_step_reason, 'last_step_finish'
        usage = reported_usage
        token_values = [value for key, value in (usage or {}).items()
                        if key in ('input_tokens', 'output_tokens', 'cached_tokens', 'reasoning_tokens')
                        and isinstance(value, (int, float))] if isinstance(usage, dict) else []
        if activity and token_values and all(value == 0 for value in token_values):
            usage = None
        completed = (exit_code == 0 and terminal is not None
                     and terminal.get('type') == 'chat.done'
                     and reason in ('stop', 'end_turn')
                     and not any(event.get('type') == 'chat.error' for event in events)
                     and records[-1] == {'type': 'stream_end', 'status': 'terminated'})
        return {'terminal': terminal, 'finish_reason': reason,
                'finish_reason_source': reason_source, 'reported_usage': reported_usage,
                'usage': usage, 'usage_missing': usage is None,
                'status': 'completed' if completed else 'failed'}

    async def run(self, instruction, environment, context):
        attempt = uuid.uuid4().hex
        directory = Path(self.logs_dir) / attempt
        directory.mkdir(parents=True, exist_ok=False)
        remote = self.remote + '/' + attempt
        binary = self.remote + '/runtime/shiju'
        timeout = self.task['agentTimeoutSec']
        result = {'request_id': attempt, 'adapter_version': self.version(),
                  'adapter_source_sha256': _ADAPTER_SOURCE_SHA256,
                  'directory': self.task['workingDirectory'],
                  'model': self.model_name, 'timeout_sec': timeout, 'status': 'failed',
                  'exit_code': None, 'terminal': None, 'usage': None, 'usage_missing': True}
        raw = ''
        started = False
        client = (shlex.quote(binary) + ' realtime --host 127.0.0.1 --port 42619 '
                  '--directory ' + shlex.quote(self.task['workingDirectory'])
                  + ' --platform 10000 --company 100000000 --output jsonl --yes ')
        try:
            preparation = await self._execute(environment, 'umask 077; mkdir -p ' + shlex.quote(remote))
            if preparation.return_code:
                raise RuntimeError('attempt directory creation failed')
            body = {'session_id': attempt, 'model': self.model_name,
                    'messages': [{'role': 'user', 'content': instruction}]}
            with tempfile.TemporaryDirectory() as temp:
                request = Path(temp) / 'request.json'
                request.write_text(json.dumps(body, ensure_ascii=False))
                await environment.upload_file(request, remote + '/request.json')
            # Start only in the task environment. Never invoke a host Gateway.
            start = ("if bash -c 'exec 3<>/dev/tcp/127.0.0.1/42619' 2>/dev/null; "
                     "then exit 1; fi; " + shlex.quote(binary) + ' --config-dir ' + shlex.quote(self.config_directory)
                     + ' gateway --host 127.0.0.1 --port 42619 > ' + remote + '/gateway.log 2>&1 & '
                     'echo $! > ' + remote + '/gateway.pid')
            started = True
            launch = await self._execute(environment, start)
            if launch.return_code:
                raise RuntimeError('Gateway launch failed')
            health = await self._execute(environment,
                "bash -c 'for ((i=0;i<30;i++)); do "
                "response=$( (exec 3<>/dev/tcp/127.0.0.1/42619; "
                "printf \"GET /v1/health HTTP/1.0\\r\\nHost: localhost\\r\\n\\r\\n\" >&3; cat <&3) 2>/dev/null); "
                "if [[ $response == *\"200 OK\"* && $response == *\"\\\"status\\\":\\\"ok\\\"\"* ]]; "
                "then exit 0; fi; sleep 1; done; exit 1'", timeout_sec=40)
            if health.return_code:
                raise RuntimeError('Gateway not ready')
            result['profile_bound'] = await self._bind_profile(environment, remote)
            command = (client + 'chat-stream --request-id ' + attempt + ' --input '
                       + remote + '/request.json > ' + remote + '/events.jsonl 2> '
                       + remote + '/client.stderr; code=$?; cat ' + remote
                       + '/events.jsonl; cat ' + remote + '/client.stderr >&2; exit "$code"')
            execution = await asyncio.wait_for(
                self._execute(environment, command, timeout_sec=timeout), timeout=timeout)
            # Harbor may merge exec diagnostics; parse only the client's JSONL file.
            await environment.download_file(remote + '/events.jsonl', directory / 'events.jsonl')
            raw = (directory / 'events.jsonl').read_text()
            result['exit_code'] = execution.return_code
            (directory / 'client.stderr').write_text(execution.stderr or '')
            result.update(self.summarize_stream(raw, execution.return_code))
            if result['status'] != 'completed':
                raise RuntimeError('Gateway request failed or did not complete normally')
        except asyncio.CancelledError:
            result['status'] = 'cancelled'
            raise
        except TimeoutError:
            result['status'] = 'timed_out'
            raise
        except Exception as error:
            result['error'] = type(error).__name__
            if isinstance(error, ValueError):
                raise
            raise NonZeroAgentExitCodeError('VibeWindow request failed: ' + type(error).__name__) from error
        finally:
            (directory / 'events.jsonl').write_text(raw)
            try:
                if started:
                    stop = await self._execute(environment, client + 'chat-stop ' + attempt)
                    result['stop_exit_code'] = stop.return_code
            except Exception as error:
                result['stop_error'] = type(error).__name__
            try:
                if started:
                    cleanup = await self._execute(environment,
                        'if [ -f ' + remote + '/gateway.pid ]; then pid=$(cat ' + remote
                        + '/gateway.pid); kill -TERM "$pid" 2>/dev/null || true; '
                        'for i in 1 2 3 4 5; do kill -0 "$pid" 2>/dev/null || exit 0; sleep 1; done; '
                        'kill -KILL "$pid" 2>/dev/null || true; fi')
                    result['cleanup_exit_code'] = cleanup.return_code
            except Exception as error:
                result['cleanup_error'] = type(error).__name__
            # Recover partial output even when Harbor cancelled the exec call.
            for name in ('events.jsonl', 'gateway.log', 'client.stderr'):
                try:
                    await environment.download_file(remote + '/' + name, directory / name)
                except (AttributeError, FileNotFoundError, RuntimeError) as error:
                    result.setdefault('archive_missing', {})[name] = type(error).__name__
            try:
                removed = await self._execute(environment, 'rm -rf -- ' + shlex.quote(self.remote))
                result['profile_cleanup_exit_code'] = removed.return_code
            except Exception as error:
                result['profile_cleanup_error'] = type(error).__name__
            if result['status'] == 'completed' and (
                    result.get('profile_cleanup_exit_code') != 0 or
                    result.get('cleanup_exit_code') != 0 or 'cleanup_error' in result):
                result['status'] = 'failed'
            (directory / 'result.json').write_text(json.dumps(result, indent=2))
            context.metadata = {**(context.metadata or {}), 'vibewindow': result}
            if result['usage'] is not None:
                for source, target in (('input_tokens', 'n_input_tokens'),
                                       ('output_tokens', 'n_output_tokens'),
                                       ('cached_tokens', 'n_cache_tokens')):
                    if source in result['usage']:
                        setattr(context, target, result['usage'][source])
        if result['status'] != 'completed':
            raise NonZeroAgentExitCodeError('Gateway cleanup failed')
