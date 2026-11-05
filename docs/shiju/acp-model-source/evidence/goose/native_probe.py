#!/usr/bin/env python3
"""Exercise a real Goose 1.57.0 binary. Only the loopback LLM is simulated."""
import argparse
import base64
import hashlib
import shutil
import json
import os
from pathlib import Path
import queue
import secrets
import shlex
import signal
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


class Fixture:
    def __init__(self, root=None):
        self.temporary = tempfile.TemporaryDirectory(prefix='goose-fixture-') if root is None else None
        root = Path(self.temporary.name) if self.temporary else root
        self.production_acceptance = self.temporary is not None
        self.control = secrets.token_hex(24)
        self.models = ('acp-fixture-primary', 'acp-fixture-switched')
        self.second_read_path = None
        self.key = secrets.token_hex(24)
        self.challenge = secrets.token_hex(16)
        self.path = root / 'challenge.txt'
        self.path.write_text(self.challenge)
        self.records = []
        self.errors = 0
        self.cancel_seen = threading.Event()
        self.auxiliary_seen = threading.Event()
        self.lock = threading.Lock()
        fixture = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def reply(self, code, body):
                data = json.dumps(body).encode()
                self.send_response(code)
                self.send_header('Content-Type', 'application/json')
                self.send_header('Content-Length', str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            def do_GET(self):
                if self.path == '/_evidence' and self.headers.get('Authorization') == 'Bearer ' + fixture.control:
                    return self.reply(200, fixture.snapshot())
                # Native context/model discovery is not an LLM completion.
                if self.path == '/v1/models':
                    return self.reply(200, {'data': [{'id': 'acp-fixture-primary'}, {'id': 'acp-fixture-switched'}]})
                if self.path.startswith('/v1/models/'):
                    return self.reply(200, {'id': self.path.rsplit('/', 1)[1], 'context_length': 128000})
                self.reply(404, {})

            def do_POST(self):
                try:
                    body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                    authenticated = self.headers.get('Authorization') == 'Bearer ' + fixture.key
                    model = body.get('model')
                    if self.path != '/v1/chat/completions' or not authenticated:
                        raise ValueError('route or authentication')
                    messages = body.get('messages', [])
                    user = str(next((m.get('content') for m in reversed(messages) if m.get('role') == 'user'), ''))
                    results = [m for m in messages if m.get('role') == 'tool' and fixture.challenge in json.dumps(m.get('content'))]
                    auxiliary = 'AUXILIARY_MODEL_PROBE' in user
                    with fixture.lock:
                        fixture.records.append({'model': model, 'path': self.path, 'authenticated': authenticated,
                                                'tool_results': len(results), 'auxiliary': auxiliary})
                    if auxiliary:
                        fixture.auxiliary_seen.set()
                    if 'CANCEL_MODEL_PROBE' in user or 'CANCEL_PROBE' in user:
                        fixture.cancel_seen.set()
                        time.sleep(2)
                    functions = [t.get('function', {}) for t in body.get('tools', [])]
                    shell = next((f for f in functions if f.get('name', '').endswith('__shell') or f.get('name') == 'shell'), None)
                    delegate = next((f for f in functions if f.get('name', '').endswith('__delegate') or f.get('name') == 'delegate'), None)
                    already_delegated = any('delegate' in str(m.get('name', '')) or ('tool_calls' in m and any('delegate' in c.get('function', {}).get('name', '') for c in m['tool_calls'])) for m in messages)
                    call = None
                    if 'DELEGATE_MODEL_PROBE' in user and not already_delegated and delegate:
                        call = (delegate['name'], {'instructions': 'AUXILIARY_MODEL_PROBE: reply briefly.', 'provider': 'openai', 'model': 'forbidden-auxiliary-model', 'extensions': [], 'max_turns': 1})
                    elif ('READ_MODEL_PROBE' in user or fixture.production_acceptance) and len(results) < 2 and shell:
                        # Actual upstream shell tool reads only the owned challenge file.
                        call = (shell['name'], {'command': 'cat -- ' + shlex.quote(str(fixture.path))})
                    if call:
                        message = {'role': 'assistant', 'content': None, 'tool_calls': [{'id': 'probe-' + secrets.token_hex(4), 'type': 'function', 'function': {'name': call[0], 'arguments': json.dumps(call[1])}}]}
                        finish = 'tool_calls'
                    else:
                        message = {'role': 'assistant', 'content': ('FIXTURE_MODEL_REPLY ' if fixture.production_acceptance else 'REAL_GOOSE_MODEL_REPLY ') + fixture.challenge}
                        finish = 'stop'
                    if body.get('stream'):
                        delta = dict(message)
                        if 'tool_calls' in delta:
                            delta['tool_calls'] = [dict(delta['tool_calls'][0], index=0)]
                        chunks = [{'id': 'probe', 'object': 'chat.completion.chunk', 'model': model,
                                   'choices': [{'index': 0, 'delta': delta, 'finish_reason': None}]},
                                  {'id': 'probe', 'object': 'chat.completion.chunk', 'model': model,
                                   'choices': [{'index': 0, 'delta': {}, 'finish_reason': finish}],
                                   'usage': {'prompt_tokens': 1, 'completion_tokens': 1, 'total_tokens': 2}}]
                        data = (''.join('data: ' + json.dumps(c) + '\n\n' for c in chunks) + 'data: [DONE]\n\n').encode()
                        self.send_response(200)
                        self.send_header('Content-Type', 'text/event-stream')
                        self.send_header('Content-Length', str(len(data)))
                        self.end_headers()
                        try:
                            self.wfile.write(data)
                        except (BrokenPipeError, ConnectionResetError):
                            pass  # Expected when cancelling the real native request.
                    else:
                        self.reply(200, {'id': 'probe', 'object': 'chat.completion', 'model': model, 'choices': [{'index': 0, 'message': message, 'finish_reason': finish}], 'usage': {'prompt_tokens': 1, 'completion_tokens': 1, 'total_tokens': 2}})
                except (ValueError, KeyError):
                    fixture.errors += 1
                    self.reply(400, {})

        self.server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.server.daemon_threads = True
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.url = 'http://127.0.0.1:' + str(self.server.server_port)

    @property
    def read_path(self):
        return str(self.path)

    @read_path.setter
    def read_path(self, value):
        self.path = Path(value)

    def snapshot(self):
        with self.lock:
            return {'requests': [dict(r, route=r['path']) for r in self.records], 'errors': self.errors}

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        self.close()

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()
        if self.temporary:
            self.temporary.cleanup()


class Client:
    def __init__(self, binary, root, env, managed):
        self.process = subprocess.Popen([str(binary), 'acp'] + (['--shiju-model-source'] if managed else []), cwd=root,
                                        env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=subprocess.DEVNULL, text=True, start_new_session=True)
        self.queue = queue.Queue()
        self.serial = 0
        self.events = []
        self.root = root
        def reader():
            for line in self.process.stdout:
                try:
                    self.queue.put(json.loads(line))
                except ValueError:
                    pass
            self.queue.put(None)
        self.reader = threading.Thread(target=reader, daemon=True)
        self.reader.start()

    def send(self, method, params, response=True):
        self.serial += 1
        message = {'jsonrpc': '2.0', 'method': method, 'params': params}
        if response:
            message['id'] = self.serial
        self.process.stdin.write(json.dumps(message) + '\n')
        self.process.stdin.flush()
        return self.serial

    def receive(self, ident, timeout=40):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            message = self.queue.get(timeout=max(0.1, deadline - time.monotonic()))
            if message is None:
                raise RuntimeError('real CLI exited')
            if 'method' in message:
                self.events.append(message)
                if 'id' in message:
                    # Native approval remains in effect; acknowledge only fixture-owned reads.
                    if message['method'] == 'session/request_permission':
                        params = message.get('params', {})
                        tool = json.dumps(params.get('toolCall', {}))
                        allowed = str(self.root / 'challenge.txt') in tool and 'cat -- ' in tool
                        option = next((o['optionId'] for o in params.get('options', []) if o.get('kind') == 'allow_once'), None)
                        outcome = {'outcome': 'selected', 'optionId': option} if allowed and option else {'outcome': 'cancelled'}
                        reply = {'jsonrpc': '2.0', 'id': message['id'], 'result': {'outcome': outcome}}
                    else:
                        reply = {'jsonrpc': '2.0', 'id': message['id'], 'error': {'code': -32601, 'message': 'Unsupported fixture client request'}}
                    self.process.stdin.write(json.dumps(reply) + '\n')
                    self.process.stdin.flush()
            elif message.get('id') == ident:
                if 'error' in message:
                    raise RuntimeError('native RPC error ' + str(message['error'].get('code')))
                return message['result']
        raise TimeoutError('native RPC timed out')

    def rpc(self, method, params):
        return self.receive(self.send(method, params))

    def close(self):
        if self.process.poll() is None:
            try:
                os.killpg(self.process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(self.process.pid, signal.SIGKILL)
            self.process.wait()
        self.reader.join(timeout=5)
        self.process.stdin.close()
        self.process.stdout.close()


def run(binary, output):
    report = {'binary_version': subprocess.check_output([str(binary), '--version'], text=True).strip(),
              'platform': os.uname().sysname + ' ' + os.uname().machine, 'modes': []}
    for mode in ('session-model', 'cli-local'):
        with tempfile.TemporaryDirectory(prefix='goose-native-proof-') as temporary:
            root = Path(temporary)
            fixture = Fixture(root)
            native = root / 'native'
            runtime = root / 'runtime'
            for path in (native / 'config', runtime):
                path.mkdir(parents=True, mode=0o700)
            native_config = json.dumps({'GOOSE_PROVIDER': 'openai', 'GOOSE_MODEL': 'acp-fixture-primary', 'OPENAI_HOST': fixture.url, 'OPENAI_BASE_PATH': 'v1/chat/completions', 'GOOSE_DISABLE_KEYRING': True})
            (native / 'config/config.yaml').write_text(native_config)
            (native / 'config/secrets.yaml').write_text(json.dumps({'OPENAI_API_KEY': fixture.key}))
            env = {key: os.environ[key] for key in ('PATH', 'HOME', 'TMPDIR') if key in os.environ}
            env['GOOSE_PATH_ROOT'] = str(native)
            managed = mode == 'session-model'
            if managed:
                (native / 'config/config.yaml').write_text(json.dumps({'GOOSE_PROVIDER': 'openai', 'GOOSE_MODEL': 'forbidden-native-model', 'OPENAI_HOST': fixture.url, 'GOOSE_DISABLE_KEYRING': True}))
                (native / 'config/secrets.yaml').write_text(json.dumps({'OPENAI_API_KEY': 'forbidden-native-key'}))
                env.update(VW_ACP_LAUNCH_REFERENCE='vw-session-model:' + hashlib.sha256((str(root) + ':primary').encode()).hexdigest(), GOOSE_PATH_ROOT=str(runtime), GOOSE_PROVIDER='openai', GOOSE_MODEL='acp-fixture-primary', SHIJU_GOOSE_MODEL_LOCK='acp-fixture-primary', OPENAI_HOST=fixture.url, OPENAI_API_KEY=fixture.key, OPENAI_BASE_PATH='v1/chat/completions', OPENAI_CUSTOM_HEADERS='', GOOSE_DISABLE_KEYRING='true')
            snapshot = (native / 'config/config.yaml').read_bytes(), (native / 'config/secrets.yaml').read_bytes()
            flow = {}
            client = None
            try:
                client = Client(binary, root, env, managed)
                init = client.rpc('initialize', {'protocolVersion': 1, 'clientCapabilities': {}, 'clientInfo': {'name': 'goose-real-proof', 'version': '1'}})
                flow['handshake'] = init.get('agentInfo', {}).get('version') == '1.57.0'
                recipe = {'version': '1.0.0', 'title': 'Offline recipe', 'description': 'Owned loopback', 'instructions': 'Reply briefly', 'settings': {'goose_provider': 'openai', 'goose_model': 'forbidden-recipe-model' if managed else 'acp-fixture-primary'}}
                link = base64.urlsafe_b64encode(json.dumps(recipe).encode()).decode().rstrip('=')
                session = client.rpc('session/new', {'cwd': str(root), 'mcpServers': [], '_meta': {'recipeDeeplink': link}})
                sid = session['sessionId']
                selected = next(o['currentValue'] for o in session['configOptions'] if o['id'] == 'model')
                flow['recipe'] = selected == 'acp-fixture-primary'
                def prompt(text):
                    return client.rpc('session/prompt', {'sessionId': sid, 'prompt': [{'type': 'text', 'text': text}]})
                flow['new'] = prompt('READ_MODEL_PROBE: read challenge twice.')['stopReason'] == 'end_turn'
                flow['second_turn'] = prompt('Reply briefly.')['stopReason'] == 'end_turn'
                prompt('DELEGATE_MODEL_PROBE: delegate once.')
                flow['auxiliary'] = fixture.auxiliary_seen.wait(10)
                pending = client.send('session/prompt', {'sessionId': sid, 'prompt': [{'type': 'text', 'text': 'CANCEL_MODEL_PROBE'}]})
                assert fixture.cancel_seen.wait(10)
                client.send('session/cancel', {'sessionId': sid}, response=False)
                flow['cancel'] = client.receive(pending)['stopReason'] == 'cancelled'
                client.close()
                if managed:
                    shutil.rmtree(runtime)
                    renewed = root / 'renewed-runtime'
                    renewed.mkdir(mode=0o700)
                    fixture.key = secrets.token_hex(24)
                    env.update(GOOSE_PATH_ROOT=str(renewed), OPENAI_API_KEY=fixture.key)
                client = Client(binary, root, env, managed)
                client.rpc('initialize', {'protocolVersion': 1, 'clientCapabilities': {}})
                client.rpc('session/load', {'sessionId': sid, 'cwd': str(root), 'mcpServers': []})
                flow['restart'] = prompt('Reply after restart.')['stopReason'] == 'end_turn'
                if managed:
                    client.close()
                    switched_root = root / 'switched'
                    switched_root.mkdir(mode=0o700)
                    env.update(VW_ACP_LAUNCH_REFERENCE='vw-session-model:' + hashlib.sha256((str(root) + ':switched').encode()).hexdigest(), GOOSE_PATH_ROOT=str(switched_root), GOOSE_MODEL='acp-fixture-switched', SHIJU_GOOSE_MODEL_LOCK='acp-fixture-switched')
                    client = Client(binary, root, env, True)
                    client.rpc('initialize', {'protocolVersion': 1, 'clientCapabilities': {}})
                    sid = client.rpc('session/new', {'cwd': str(root), 'mcpServers': []})['sessionId']
                    flow['switch'] = prompt('Reply after switch.')['stopReason'] == 'end_turn'
                else:
                    flow['switch'] = True
                flow['isolated'] = snapshot == ((native / 'config/config.yaml').read_bytes(), (native / 'config/secrets.yaml').read_bytes())
                records = fixture.records.copy()
                flow['wire_models'] = all(r['model'] in (('acp-fixture-primary', 'acp-fixture-switched') if managed else ('acp-fixture-primary', 'forbidden-auxiliary-model')) for r in records)
                flow['tool_rounds'] = any(r['tool_results'] >= 2 for r in records)
                # Native local mode must honor its explicitly requested auxiliary model.
                flow['auxiliary_source'] = any(r['auxiliary'] and r['model'] == ('acp-fixture-primary' if managed else 'forbidden-auxiliary-model') for r in records)
                report['modes'].append({'mode': mode, 'flow': flow, 'passed': all(flow.values()) and fixture.errors == 0, 'requests': records, 'errors': fixture.errors})
            except Exception as error:
                report['modes'].append({'mode': mode, 'flow': flow, 'passed': False, 'reason': str(error), 'requests': fixture.records.copy(), 'errors': fixture.errors})
            finally:
                if client:
                    client.close()
                fixture.close()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))
    return 0 if len(report['modes']) == 2 and all(m['passed'] for m in report['modes']) else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    raise SystemExit(run(args.binary.resolve(), args.output))
