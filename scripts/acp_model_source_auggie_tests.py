"""Exercise the reviewed Auggie runtime's real ACP/file-tool loop against a local HTTP fixture.
Usage: python3 scripts/acp_model_source_auggie_tests.py /absolute/path/to/augment.mjs
"""
import http.server
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import tempfile
import threading
import time

entry = Path(sys.argv[1]).resolve(strict=True)
with tempfile.TemporaryDirectory(prefix='auggie-native-test-') as directory:
    root = Path(directory)
    root.chmod(0o700)
    requests = []

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers['content-length'])))
            if self.path != '/v1/chat/completions':
                self.send_response(404)
                self.end_headers()
                self.wfile.write(b'{}')
                return
            requests.append(body)
            assert self.headers['Authorization'] == 'Bearer fixture'
            assert body['model'] == 'selected'
            if len(requests) == 1:
                delta = {'reasoning_content': 'fixture reasoning', 'tool_calls': [{'index': 0, 'id': 'save-fixture', 'type': 'function', 'function': {
                    'name': 'save-file', 'arguments': json.dumps({'path': 'created.txt', 'file_content': 'native tool result'})}}]}
                reason = 'tool_calls'
            else:
                delta, reason = {'content': '工具已完成'}, 'stop'
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            self.wfile.write(('data: ' + json.dumps({'choices': [{'delta': delta, 'finish_reason': reason}]}) +
                              '\n\ndata: [DONE]\n\n').encode())

    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    endpoint = f'http://127.0.0.1:{server.server_port}/'
    (root / 'model.json').write_text(json.dumps({'model': 'selected', 'endpoint': endpoint, 'token': 'fixture', 'outputLimit': 4096}))
    (root / 'auth.json').write_text(json.dumps({'accessToken': 'fixture', 'tenantURL': endpoint, 'scopes': ['email']}))
    workspace = root / 'workspace'
    workspace.mkdir()
    env = {k: v for k, v in os.environ.items() if k in ('PATH', 'HOME', 'TMPDIR')}
    env.update(SHIJU_AUGGIE_MODEL_CONFIG=str(root / 'model.json'), AUGMENT_DISABLE_AUTO_UPDATE='1')
    with (root / 'stderr').open('w') as stderr:
        process = subprocess.Popen(['node', str(entry), '--acp', '--model', 'selected', '--augment-cache-dir', str(root / 'cache'),
                                    '--augment-session-json', str(root / 'auth.json')], cwd=workspace, env=env,
                                   stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr, text=True)
        incoming = queue.Queue()

        def read():
            for line in process.stdout:
                incoming.put(json.loads(line))

        threading.Thread(target=read, daemon=True).start()

        def send(message):
            process.stdin.write(json.dumps({'jsonrpc': '2.0', **message}) + '\n')
            process.stdin.flush()

        send({'id': 1, 'method': 'initialize', 'params': {'protocolVersion': 1, 'clientCapabilities': {}}})
        completed = False
        updates = []
        try:
            deadline = time.monotonic() + 60
            while time.monotonic() < deadline:
                try:
                    message = incoming.get(timeout=1)
                except queue.Empty:
                    assert process.poll() is None, 'Native Auggie exited before completion'
                    continue
                assert 'error' not in message, message.get('error')
                if message.get('id') == 1:
                    send({'id': 2, 'method': 'session/new', 'params': {'cwd': str(workspace), 'mcpServers': []}})
                elif message.get('id') == 2:
                    assert message['result']['models']['currentModelId'] == 'selected'
                    send({'id': 3, 'method': 'session/prompt', 'params': {'sessionId': message['result']['sessionId'],
                          'prompt': [{'type': 'text', 'text': 'Create created.txt using save-file then finish.'}]}})
                elif message.get('id') == 3:
                    assert message['result']['stopReason'] == 'end_turn'
                    completed = True
                    break
                elif message.get('method') == 'session/update':
                    updates.append(message['params']['update'])
                elif message.get('method') == 'session/request_permission':
                    option = next(o for o in message['params']['options'] if o['kind'] == 'allow_once')
                    send({'id': message['id'], 'result': {'outcome': {'outcome': 'selected', 'optionId': option['optionId']}}})
            assert completed, 'Native Auggie did not complete within 60 seconds'
            assert (workspace / 'created.txt').read_text().strip() == 'native tool result'
            assert len(requests) == 2
            assistant = next(m for m in requests[1]['messages'] if m['role'] == 'assistant')
            assert assistant['reasoning_content'] == 'fixture reasoning'
            results = [m for m in requests[1]['messages'] if m['role'] == 'tool']
            assert len(results) == 1 and results[0]['tool_call_id'] == 'save-fixture'
            assert 'File saved' in results[0]['content']
            assert any(u.get('status') == 'completed' and u.get('toolCallId') == 'save-fixture' for u in updates)
            assert any(u.get('sessionUpdate') == 'agent_message_chunk' and u['content'].get('text') == '工具已完成' for u in updates)
            print('PASS: native ACP model selection, file tool, continuation and Chinese stream')
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            server.shutdown()
