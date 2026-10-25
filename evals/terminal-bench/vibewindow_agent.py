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

from harbor.agents.installed.base import BaseInstalledAgent


class VibeWindowAgent(BaseInstalledAgent):
    def __init__(self, logs_dir, *, task_id, runtime_dir=None,
                 config_directory=None, model_name=None, **kwargs):
        manifest = json.loads(Path(__file__).with_name('manifest.json').read_text())
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
        return 'task27-harbor-0.24.0'

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

    async def run(self, instruction, environment, context):
        attempt = uuid.uuid4().hex
        directory = Path(self.logs_dir) / attempt
        directory.mkdir(parents=True, exist_ok=False)
        remote = self.remote + '/' + attempt
        binary = self.remote + '/runtime/shiju'
        timeout = self.task['agentTimeoutSec']
        result = {'request_id': attempt, 'directory': self.task['workingDirectory'],
                  'model': self.model_name, 'timeout_sec': timeout, 'status': 'failed',
                  'exit_code': None, 'terminal': None, 'usage': None, 'usage_missing': True}
        raw = ''
        started = False
        client = (shlex.quote(binary) + ' realtime --host 127.0.0.1 --port 42619 '
                  '--directory ' + shlex.quote(self.task['workingDirectory']) + ' --output jsonl --yes ')
        try:
            preparation = await self._execute(environment, 'mkdir -p ' + shlex.quote(remote))
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
            command = (client + 'chat-stream --request-id ' + attempt + ' --input '
                       + remote + '/request.json > ' + remote + '/events.jsonl 2> '
                       + remote + '/client.stderr; code=$?; cat ' + remote
                       + '/events.jsonl; cat ' + remote + '/client.stderr >&2; exit "$code"')
            execution = await asyncio.wait_for(
                self._execute(environment, command, timeout_sec=timeout), timeout=timeout)
            raw = execution.stdout or ''
            result['exit_code'] = execution.return_code
            (directory / 'client.stderr').write_text(execution.stderr or '')
            try:
                records = [json.loads(line) for line in raw.splitlines() if line.strip()]
                if any(not isinstance(record, dict) for record in records):
                    raise ValueError('expected object records')
            except ValueError as error:
                raise RuntimeError('invalid Gateway JSONL') from error
            events = [record['event'] for record in records if 'event' in record]
            for event in events:
                if event.get('usage') is not None:
                    result['usage'] = event['usage']
                    result['usage_missing'] = False
                if event.get('type') in ('chat.done', 'chat.error'):
                    result['terminal'] = event
            terminal = result['terminal'] or {}
            if (execution.return_code or any(e.get('type') == 'chat.error' for e in events)
                    or terminal.get('type') != 'chat.done'
                    or terminal.get('finish_reason') not in ('stop', 'end_turn')
                    or not records or records[-1] != {'type': 'stream_end', 'status': 'terminated'}):
                raise RuntimeError('Gateway request failed or did not complete normally')
            result['status'] = 'completed'
        except asyncio.CancelledError:
            result['status'] = 'cancelled'
            raise
        except TimeoutError:
            result['status'] = 'timed_out'
            raise
        except Exception as error:
            result['error'] = type(error).__name__
            raise
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
            if result['status'] == 'completed' and (
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
            raise RuntimeError('Gateway cleanup failed')
