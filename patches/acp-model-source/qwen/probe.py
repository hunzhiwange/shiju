#!/usr/bin/env python3
"""Supplementary real Qwen lifecycle probe; never a substitute for Rust acceptance."""
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
parser.add_argument('--auxiliary', action='store_true')
args = parser.parse_args()
if sys.platform != 'darwin' or not shutil.which('sandbox-exec'):
    raise SystemExit('This supplementary isolation probe requires macOS sandbox-exec')
cli = str(args.cli.absolute())
base = {key: os.environ[key] for key in ('PATH', 'HOME', 'TMPDIR') if key in os.environ}
with tempfile.TemporaryDirectory(prefix='qwen-version-') as version_root:
    version_env = dict(base, QWEN_HOME=version_root, QWEN_RUNTIME_DIR=version_root)
    version = subprocess.run([cli, '--version'], env=version_env, capture_output=True, timeout=20)
if version.returncode or version.stdout.strip() != b'0.26.7':
    raise SystemExit('Expected fixed real Qwen 0.26.7')
with tempfile.TemporaryDirectory(prefix='qwen-lifecycle-') as temporary, ModelFixture() as fixture:
    scope = Path(temporary)
    scope.chmod(0o700)
    history = scope / 'history'
    history.mkdir(mode=0o700)
    native = scope / 'native'
    native.mkdir(mode=0o700)
    fixture.read_path = str(scope / 'challenge.txt')
    Path(fixture.read_path).write_text(fixture.challenge)
    fixture.second_read_path = str(scope / 'challenge-second.txt')
    Path(fixture.second_read_path).write_text(fixture.challenge)
    (native / 'settings.json').write_text(json.dumps({
        'security': {'auth': {'selectedType':'openai', 'apiKey':fixture.key,'baseUrl':fixture.url+'/v1'}},
        'model':{'name':fixture.models[0]}, 'tools':{'toolSearch':{'enabled':False}}, 'general':{'enableAutoUpdate':False}, 'telemetry':{'enabled':False}
    }))
    sandbox = ('(version 1)(allow default)(deny network*)'
               '(allow network-outbound (remote ip "localhost:*"))'
               '(deny file-read* file-write* (subpath ' + json.dumps(str(Path.home() / '.qwen')) + ') (literal ' + json.dumps(str(Path.home()/'.env')) + '))')
    fixture.probe_auxiliary = args.auxiliary
    fixture.auxiliary_model = None
    session = None

    def turn(model, cancel=False):
        global session
        with tempfile.TemporaryDirectory(prefix='route-', dir=scope) as temporary_route:
            route = Path(temporary_route)
            env = dict(base, QWEN_HOME=str(native))
            command = ['sandbox-exec', '-p', sandbox, cli, '--acp', '--experimental-skills', '--approval-mode', 'yolo']
            env.update(QWEN_RUNTIME_DIR=str(history))
            if args.mode == 'session-model':
                alias = 'shiju-offline-private-alias'
                generation = {'model':alias,'contextWindowSize':32768,'samplingParams':{'max_tokens':4096},'extra_body':{'model':model}}
                settings = {'security':{'auth':{'selectedType':'openai'}},'model':{'name':alias},
                    'tools':{'toolSearch':{'enabled':False}},'modelProviders':{'openai':[{'id':alias,'baseUrl':fixture.url+'/v1','envKey':'OPENAI_API_KEY','generationConfig':generation}]},
                    'general':{'enableAutoUpdate':False},'telemetry':{'enabled':False}}
                path = route/'settings.json';path.write_text(json.dumps(settings))
                env.update(QWEN_HOME=str(route), QWEN_CODE_SYSTEM_SETTINGS_PATH=str(path), OPENAI_API_KEY=fixture.key,
                    OPENAI_BASE_URL=fixture.url+'/v1', OPENAI_MODEL=alias,SHIJU_QWEN_MODEL_LOCK=model,SHIJU_QWEN_GENERATION_CONFIG=json.dumps(generation))
                command += ['--shiju-model-source','--auth-type','openai','--model',alias,'--openai-base-url',fixture.url+'/v1']
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
                'clientInfo': {'name': 'real-qwen-lifecycle', 'version': '1'}}, 1)
            selector = selectors.DefaultSelector()
            selector.register(process.stdout, selectors.EVENT_READ)
            selector.register(process.stderr, selectors.EVENT_READ)
            buffers = {process.stdout: b'', process.stderr: b''}
            replied = completed = cancelled = False
            auth_locked = args.mode != 'session-model'
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
                            if message.get('method') and 'id' in message:
                                process.stdin.write((json.dumps({'jsonrpc':'2.0','id':message['id'],'error':{'code':-32601,'message':'Method not found'}})+'\n').encode()); process.stdin.flush()
                            if message.get('id') == 90 and 'error' in message:
                                auth_locked = True
                                continue
                            if 'error' in message:
                                raise RuntimeError('Real Qwen returned a protocol error')
                            if message.get('id') == 1 and 'result' in message:
                                if args.mode == 'session-model':
                                    send('authenticate', {'methodId':'qwen-oauth'}, 90)
                                params = {'cwd': str(scope), 'mcpServers': []}
                                if session:
                                    params['sessionId'] = session
                                send('session/load' if session else 'session/new', params, 2)
                            elif message.get('id') == 2 and 'result' in message:
                                session = message['result'].get('sessionId', session)
                                send('session/prompt', {'sessionId': session, 'prompt': [{
                                    'type': 'text', 'text': 'CANCEL_PROBE: read challenge.txt' if cancel
                                    else ('AUXILIARY_PROBE: Delegate reading the challenge files to a helper.' if args.auxiliary else 'Read challenge.txt twice and return its contents.')}]}, 3)
                            elif message.get('id') == 3 and 'result' in message:
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
            return auth_locked and completed and ((cancelled and stop_reason == 'cancelled') if cancel else replied) and len(fixture.snapshot()['requests']) > before

    flow = {}
    for label in (('new',) if args.auxiliary else ('new', 'second_turn', 'cancel', 'restart', 'switch')):
        model = fixture.models[1] if label == 'switch' and args.mode == 'session-model' else fixture.models[0]
        flow[label] = turn(model, cancel=label == 'cancel')
    observed = fixture.snapshot()
    passed = all(flow.values()) and observed['errors'] == 0 and any(r['tool_results'] >= 2 for r in observed['requests'])
    if args.auxiliary:
        passed &= any(r['auxiliary'] for r in observed['requests'])
    if args.mode == 'session-model' and not args.auxiliary:
        passed &= any(r['model'] == fixture.models[1] for r in observed['requests'])
    report = {'evidence_kind': 'direct-real-cli-supplementary', 'mode': args.mode,
              'version': '0.26.7', 'platform': 'Darwin-' + platform.machine(), 'passed': passed, 'auxiliary_required': args.auxiliary,
              'native_qwen_directory_denied': True, 'flow': flow, 'requests': observed}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if passed else 1)
