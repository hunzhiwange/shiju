"""Run the installed reviewed Amp engine against an owned model fixture."""
import http.server, threading, json, subprocess, os, select, time, pathlib, tempfile, signal, shlex
root = pathlib.Path(tempfile.mkdtemp(prefix='amp-native-'))
turn = 0
calls = []
permissions = []
updates = []
deny = '--deny' in __import__('sys').argv
mode = 'command' if '--command' in __import__('sys').argv else 'read' if '--read' in __import__('sys').argv else 'edit' if '--edit' in __import__('sys').argv else 'write'
force_status = 0
terminal_calls = []
(root / 'challenge.txt').write_text('disk-hidden-sentinel')
(root / 'edit.txt').write_text('before')
(root / 'AGENTS.md').write_text('ambient-guidance-sentinel')
(root / 'state').mkdir(mode=0o700)
for config in [root / '.amp/settings.json', root / 'state/config/amp/settings.json']:
    config.parent.mkdir(parents=True, exist_ok=True)
    config.write_text(json.dumps({'amp.mcpServers': {'unselected': {'command': '/bin/sh', 'args': ['-c', 'touch ' + shlex.quote(str(root / 'ambient-executed'))]}}}))
delay_next_request = False
cancel_seen = threading.Event()

class Handler(http.server.BaseHTTPRequestHandler):

    def log_message(self, *a):
        pass

    def do_POST(self):
        global turn, delay_next_request
        b = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        calls.append((self.path, b.get('model'), self.headers.get('Authorization') == 'Bearer fixture-model-token'))
        assert 'ambient-guidance-sentinel' not in json.dumps(b)
        assert 'disk-hidden-sentinel' not in json.dumps(b)
        if force_status:
            self.send_response(force_status)
            self.send_header('content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'type': 'error', 'error': {'type': 'authentication_error' if force_status == 401 else 'not_found_error', 'message': 'fixture rejected'}}).encode())
            return
        if not b.get('stream'):
            self.send_response(200)
            self.send_header('content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'id': 'title', 'type': 'message', 'role': 'assistant', 'model': b['model'], 'content': [{'type': 'tool_use', 'id': 'title', 'name': 'set_title', 'input': {'title': 'Probe'}}], 'stop_reason': 'tool_use', 'usage': {'input_tokens': 1, 'output_tokens': 1}}).encode())
            return
        if delay_next_request:
            delay_next_request = False
            cancel_seen.set()
            time.sleep(1)
        turn += 1
        self.send_response(200)
        self.send_header('content-type', 'text/event-stream')
        self.end_headers()
        result_block = b['messages'][-1].get('content', [])
        is_result = isinstance(result_block, list) and any((c.get('type') == 'tool_result' for c in result_block))
        if is_result and mode == 'read' and (not deny):
            assert 'host-read-marker' in json.dumps(result_block)
        block = {'type': 'text', 'text': ''} if is_result else {'type': 'tool_use', 'id': f'write-{turn}', 'name': {'command': 'Bash', 'read': 'Read', 'write': 'create_file', 'edit': 'edit_file'}[mode], 'input': {}}
        delta = {'type': 'text_delta', 'text': 'OK'} if is_result else {'type': 'input_json_delta', 'partial_json': json.dumps({'cmd': 'printf command-marker > ' + shlex.quote(str(root / 'command.txt'))} if mode == 'command' else {'path': str(root / 'challenge.txt')} if mode == 'read' else {'path': str(root / 'edit.txt'), 'old_str': (root / 'edit.txt').read_text(), 'new_str':'after$&literal'} if mode == 'edit' else {'path': str(root / 'written.txt'), 'content': 'host-write'})}
        stop = 'end_turn' if is_result else 'tool_use'
        events = [('message_start', {'type': 'message_start', 'message': {'id': f'local-{turn}', 'type': 'message', 'role': 'assistant', 'model': b['model'], 'content': [], 'stop_reason': None, 'stop_sequence': None, 'usage': {'input_tokens': 1, 'output_tokens': 0}}}), ('content_block_start', {'type': 'content_block_start', 'index': 0, 'content_block': block}), ('content_block_delta', {'type': 'content_block_delta', 'index': 0, 'delta': delta}), ('content_block_stop', {'type': 'content_block_stop', 'index': 0}), ('message_delta', {'type': 'message_delta', 'delta': {'stop_reason': stop, 'stop_sequence': None}, 'usage': {'output_tokens': 1}}), ('message_stop', {'type': 'message_stop'})]
        try:
            for (e, payload) in events:
                self.wfile.write(('event: ' + e + '\ndata: ' + json.dumps(payload) + '\n\n').encode())
        except (BrokenPipeError, ConnectionResetError):
            pass
s = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
threading.Thread(target=s.serve_forever, daemon=True).start()
env = {'PATH': '/opt/homebrew/bin:/usr/bin:/bin', 'HOME': str(root), 'AMP_ACP_STATE_DIR': str(root / 'state'), 'SHIJU_AMP_RELAY_URL': f'http://127.0.0.1:{s.server_port}/', 'SHIJU_AMP_BINDING': 'fixture-scope', 'SHIJU_AMP_MODEL': 'selected-model', 'SHIJU_AMP_RELAY_TOKEN': 'fixture-model-token'}
entry = os.environ['SHIJU_AMP_TEST_ENTRY']
p = subprocess.Popen([entry], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=(root / 'stderr').open('w'), env=env, start_new_session=True)
buffer = b''

def call(i, m, params):
    global buffer
    p.stdin.write((json.dumps({'jsonrpc': '2.0', 'id': i, 'method': m, 'params': params}) + '\n').encode())
    p.stdin.flush()
    end = time.time() + 35
    while time.time() < end:
        if b'\n' not in buffer:
            if not select.select([p.stdout], [], [], end - time.time())[0]:
                break
            data = os.read(p.stdout.fileno(), 65536)
            if not data:
                break
            buffer += data
            continue
        (line, buffer) = buffer.split(b'\n', 1)
        j = json.loads(line)
        if j.get('method') == 'session/update':
            updates.append(j['params']['update'].get('sessionUpdate'))
        if 'method' in j and 'id' in j:
            method = j['method']
            params = j['params']
            if method == 'session/request_permission':
                permissions.append(method)
                result = {'outcome': {'outcome': 'selected', 'optionId': 'reject_once' if deny else 'allow_once'}}
            elif method == 'fs/read_text_file':
                assert mode in ('read','edit') and (not deny)
                result = {'content': 'host-read-marker' if mode == 'read' else (root/'edit.txt').read_text()}
            elif method == 'terminal/create':
                assert mode == 'command' and (not deny)
                terminal_calls.append(method)
                completed = subprocess.run([params['command']] + params['args'], cwd=params['cwd'], capture_output=True, text=True, timeout=3)
                assert completed.returncode == 0
                result = {'terminalId': 'fixture-terminal'}
            elif method == 'terminal/wait_for_exit':
                result = {'exitCode': 0}
            elif method == 'terminal/output':
                result = {'output': '', 'truncated': False, 'exitStatus': {'exitCode': 0}}
            elif method in ('terminal/release', 'terminal/kill'):
                result = {}
            elif method == 'fs/write_text_file':
                assert not deny
                pathlib.Path(params['path']).write_text(params['content'])
                result = {}
            else:
                raise RuntimeError('Unexpected callback ' + method)
            p.stdin.write((json.dumps({'jsonrpc': '2.0', 'id': j['id'], 'result': result}) + '\n').encode())
            p.stdin.flush()
            continue
        if j.get('id') == i:
            return j
    raise RuntimeError(m + ' no response ' + (root / 'stderr').read_text()[-2200:])
try:
    initialized = call(1, 'initialize', {'protocolVersion': 1, 'clientCapabilities': {'fs': {'readTextFile': True, 'writeTextFile': True}, 'terminal': True}})
    assert initialized['result']['authMethods'] == []
    rejected = call(20, 'session/new', {'cwd': str(root), 'mcpServers': [{'name': 'unselected', 'command': '/bin/sh', 'args': [], 'env': []}]})
    assert 'error' in rejected
    new = call(2, 'session/new', {'cwd': str(root), 'mcpServers': []})
    sid = new['result']['sessionId']
    for i in [3, 4]:
        response = call(i, 'session/prompt', {'sessionId': sid, 'prompt': [{'type': 'text', 'text': 'Reply OK @'+str(root/'challenge.txt')}]})
        assert response.get('result', {}).get('stopReason') == 'end_turn', response
    assert len(list((root / 'state/threads').glob('*.json'))) == 1
    os.killpg(p.pid, signal.SIGTERM)
    p.wait(timeout=4)
    p = subprocess.Popen([entry], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=(root / 'restart-stderr').open('w'), env=env, start_new_session=True)
    buffer = b''
    assert 'result' in call(5, 'initialize', {'protocolVersion': 1, 'clientCapabilities': {'fs': {'readTextFile': True, 'writeTextFile': True}, 'terminal': True}})
    before_history = len(updates)
    loaded = call(6, 'session/load', {'sessionId': sid, 'cwd': str(root), 'mcpServers': []})
    assert 'result' in loaded, loaded
    assert 'agent_message_chunk' in updates[before_history:], 'load must replay persisted agent messages'
    resumed = call(7, 'session/prompt', {'sessionId': sid, 'prompt': [{'type': 'text', 'text': 'Reply OK @'+str(root/'challenge.txt')}]})
    assert resumed.get('result', {}).get('stopReason') == 'end_turn', resumed
    delay_next_request = True

    def cancel_active():
        assert cancel_seen.wait(15)
        p.stdin.write((json.dumps({'jsonrpc': '2.0', 'method': 'session/cancel', 'params': {'sessionId': sid}}) + '\n').encode())
        p.stdin.flush()
    cancel_thread = threading.Thread(target=cancel_active)
    cancel_thread.start()
    cancelled = call(8, 'session/prompt', {'sessionId': sid, 'prompt': [{'type': 'text', 'text': 'Cancel this turn'}]})
    cancel_thread.join(timeout=2)
    assert cancelled.get('result', {}).get('stopReason') == 'cancelled', cancelled
    after_cancel = call(9, 'session/prompt', {'sessionId': sid, 'prompt': [{'type': 'text', 'text': 'Continue after cancellation'}]})
    assert after_cancel.get('result', {}).get('stopReason') == 'end_turn', after_cancel
    changed = call(10, 'session/set_config_option', {'sessionId': sid, 'configId': 'amp-mode', 'value': 'other'})
    assert 'error' in changed
    changed = call(11, 'session/set_mode', {'sessionId': sid, 'modeId': 'bypass'})
    assert 'error' in changed
    force_status = 401
    bad_auth = call(12, 'session/prompt', {'sessionId': sid, 'prompt': [{'type': 'text', 'text': 'Invalid provider authentication'}]})
    assert 'error' in bad_auth
    force_status = 404
    another = call(13, 'session/new', {'cwd': str(root), 'mcpServers': []})['result']['sessionId']
    bad_model = call(14, 'session/prompt', {'sessionId': another, 'prompt': [{'type': 'text', 'text': 'Invalid provider model'}]})
    assert 'error' in bad_model
    print('completed_turns', 4, 'restart_load', True, 'cancel_then_continue', True, 'auth_and_model_errors', True)
    assert calls and all((c == ('/v1/messages', 'selected-model', True) for c in calls))
    print('model_requests', len(calls))
    print('permission_callbacks', len(permissions), 'file_exists', (root / 'written.txt').exists())
    assert len(permissions) == 4
    assert (root / 'written.txt').exists() == (not deny and mode == 'write')
    assert (root / 'command.txt').exists() == (not deny and mode == 'command')
    assert (root/'edit.txt').read_text() == ('after$&literal' if mode == 'edit' and not deny else 'before')
    assert not (root / 'ambient-executed').exists()
    assert len(terminal_calls) == (4 if mode == 'command' and (not deny) else 0)
    print('root', root)
finally:
    os.killpg(p.pid, signal.SIGTERM)
    try:
        p.wait(timeout=4)
    except subprocess.TimeoutExpired:
        os.killpg(p.pid, signal.SIGKILL)
        p.wait()
    s.shutdown()
