#!/usr/bin/env python3
"""Supplementary real iFlow lifecycle probe; never a substitute for Rust acceptance."""
import argparse
import json
import os
import platform
from pathlib import Path
import selectors
import shutil
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'scripts'))
from acp_model_source_fixture import ModelFixture

parser = argparse.ArgumentParser()
parser.add_argument('--cli', required=True, type=Path)
parser.add_argument('--mode', required=True, choices=['session-model', 'cli-local'])
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
if sys.platform != 'darwin' or not shutil.which('sandbox-exec'):
    raise SystemExit('This supplementary isolation probe requires macOS sandbox-exec')
cli = str(args.cli.absolute())
base = {key: os.environ[key] for key in ('PATH', 'HOME', 'TMPDIR') if key in os.environ}
version = subprocess.run([cli, '--version'], env=base, capture_output=True, timeout=20)
if version.returncode or version.stdout.strip() != b'0.5.19':
    raise SystemExit('Expected fixed real iFlow 0.5.19')
with tempfile.TemporaryDirectory(prefix='iflow-lifecycle-') as temporary, ModelFixture() as fixture:
    scope = Path(temporary)
    scope.chmod(0o700)
    history = scope / 'history'
    history.mkdir(mode=0o700)
    native = scope / 'native'
    native.mkdir(mode=0o700)
    fixture.read_path = str(scope / 'challenge.txt')
    Path(fixture.read_path).write_text(fixture.challenge)
    (native / 'settings.json').write_text(json.dumps({
        'apiKey': fixture.key, 'baseUrl': fixture.url + '/v1',
        'modelName': fixture.models[0], 'selectedAuthType': 'openai-compatible',
        'disableAutoUpdate': True, 'usageStatisticsEnabled': False,
    }))
    sandbox = ('(version 1)(allow default)(deny network*)'
               '(allow network-outbound (remote ip "localhost:*"))'
               '(deny file-read* file-write* (subpath ' + json.dumps(str(Path.home() / '.iflow')) + '))')
    session = None

    def turn(model, cancel=False):
        global session
        with tempfile.TemporaryDirectory(prefix='route-', dir=scope) as temporary_route:
            route = Path(temporary_route)
            env = dict(base, IFLOW_HOME=str(native))
            command = ['sandbox-exec', '-p', sandbox, cli, '--experimental-acp']
            if args.mode == 'session-model':
                # Deliberate native conflict: canonical environment must win.
                (route / 'settings.json').write_text(json.dumps({
                    'apiKey': 'native-must-not-be-used', 'modelName': 'forbidden-native-model',
                    'baseUrl': fixture.url + '/v1', 'selectedAuthType': 'oauth-iflow',
                }))
                env.update(IFLOW_HOME=str(route), IFLOW_apiKey=fixture.key,
                           IFLOW_baseUrl=fixture.url + '/v1', IFLOW_modelName=model,
                           IFLOW_selectedAuthType='openai-compatible',
                           IFLOW_disableAutoUpdate='true', IFLOW_usageStatisticsEnabled='false',
                           SHIJU_IFLOW_MODEL_LOCK=model, SHIJU_IFLOW_HISTORY_HOME=str(history))
                command.append('--shiju-model-source')
            process = subprocess.Popen(command, cwd=scope, env=env, stdin=subprocess.PIPE,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
            def send(method, params, request_id=None):
                value = {'jsonrpc': '2.0', 'method': method, 'params': params}
                if request_id is not None:
                    value['id'] = request_id
                process.stdin.write((json.dumps(value) + '\n').encode())
                process.stdin.flush()
            send('initialize', {'protocolVersion': 1, 'clientCapabilities': {
                'fs': {'readTextFile': False, 'writeTextFile': False}},
                'clientInfo': {'name': 'real-iflow-lifecycle', 'version': '1'}}, 1)
            selector = selectors.DefaultSelector()
            selector.register(process.stdout, selectors.EVENT_READ)
            selector.register(process.stderr, selectors.EVENT_READ)
            buffers = {process.stdout: b'', process.stderr: b''}
            replied = completed = cancelled = False
            stop_reason = None
            before = len(fixture.snapshot()['requests'])
            started = time.monotonic()
            try:
                while time.monotonic() - started < 45 and not completed:
                    if cancel and not cancelled and session and len(fixture.snapshot()['requests']) > before:
                        send('session/cancel', {'sessionId': session})
                        cancelled = True
                    for ready, _ in selector.select(.1):
                        data = os.read(ready.fileobj.fileno(), 65536)
                        if not data:
                            selector.unregister(ready.fileobj)
                            continue
                        buffers[ready.fileobj] += data
                        if ready.fileobj != process.stdout:
                            continue
                        while b'\n' in buffers[process.stdout]:
                            raw, buffers[process.stdout] = buffers[process.stdout].split(b'\n', 1)
                            try:
                                message = json.loads(raw)
                            except json.JSONDecodeError:
                                continue
                            if not isinstance(message, dict):
                                continue
                            replied |= fixture.challenge in json.dumps(message)
                            if 'error' in message:
                                raise RuntimeError('Real iFlow returned a protocol error')
                            if message.get('id') == 1:
                                params = {'cwd': str(scope), 'mcpServers': []}
                                if session:
                                    params['sessionId'] = session
                                send('session/load' if session else 'session/new', params, 2)
                            elif message.get('id') == 2:
                                session = message['result']['sessionId']
                                send('session/prompt', {'sessionId': session, 'prompt': [{
                                    'type': 'text', 'text': 'CANCEL_PROBE: read challenge.txt' if cancel
                                    else 'Read challenge.txt twice and return its contents.'}]}, 3)
                            elif message.get('id') == 3:
                                completed = True
                                stop_reason = message.get('result', {}).get('stopReason')
                            elif message.get('method') == 'session/request_permission':
                                # The fixture only asks for workspace reads; unexpected requests fail.
                                raise RuntimeError('Unexpected permission request')
                    if process.poll() is not None:
                        break
            finally:
                selector.close()
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
                for stream in (process.stdin, process.stdout, process.stderr):
                    stream.close()
            if args.mode == 'session-model':
                files = list(history.glob('*.json'))
                if not files or any(fixture.key.encode() in path.read_bytes() for path in files):
                    raise RuntimeError('Missing history or credentials persisted')
            return completed and ((cancelled and stop_reason == 'cancelled') if cancel else replied) and len(fixture.snapshot()['requests']) > before

    flow = {}
    for label in ('new', 'second_turn', 'cancel', 'restart', 'switch'):
        model = fixture.models[1] if label == 'switch' and args.mode == 'session-model' else fixture.models[0]
        flow[label] = turn(model, cancel=label == 'cancel')
    observed = fixture.snapshot()
    passed = all(flow.values()) and observed['errors'] == 0 and any(r['tool_results'] >= 2 for r in observed['requests'])
    if args.mode == 'session-model':
        passed &= any(r['model'] == fixture.models[1] for r in observed['requests'])
    report = {'evidence_kind': 'direct-real-cli-supplementary', 'mode': args.mode,
              'version': '0.5.19', 'platform': 'Darwin-' + platform.machine(), 'passed': passed,
              'native_iflow_directory_denied': True, 'flow': flow, 'requests': observed}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if passed else 1)
