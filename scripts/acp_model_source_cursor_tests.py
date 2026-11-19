"""Exercise the reviewed Cursor runtime's real ACP/file-tool loop against a local HTTP fixture.
Usage: python3 scripts/acp_model_source_cursor_tests.py /absolute/path/to/cursor-agent [--deny] [--past-old-limit | --step-limit | --refuse-final-tool]
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
deny = '--deny' in sys.argv[2:]
refuse_final_tool = '--refuse-final-tool' in sys.argv[2:]
at_step_limit = refuse_final_tool or '--step-limit' in sys.argv[2:]
tool_count = 2000 if at_step_limit else 257 if '--past-old-limit' in sys.argv[2:] else 1
with tempfile.TemporaryDirectory(prefix='cursor-native-test-') as directory:
    root = Path(directory)
    root.chmod(0o700)
    request_count = 0
    first_followup = None
    latest_request = None
    final_tool_sent = threading.Event()

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            global request_count, first_followup, latest_request
            body = json.loads(self.rfile.read(int(self.headers['content-length'])))
            if self.path != '/v1/chat/completions':
                self.send_response(404)
                self.end_headers()
                self.wfile.write(b'{}')
                return
            request_count += 1
            if request_count == 2:
                first_followup = body
            latest_request = body
            assert self.headers['Authorization'] == 'Bearer fixture'
            assert body['model'] == 'selected'
            if request_count == tool_count + 1:
                assert sum(m['role'] == 'tool' for m in body['messages']) == tool_count
                if at_step_limit:
                    assert body.get('tool_choice') == 'none'
            if request_count <= tool_count or refuse_final_tool:
                call_id = 'save-fixture' if request_count == 1 else f'save-fixture-{request_count}'
                delta = {'reasoning_content': 'fixture reasoning', 'tool_calls': [{'index': 0, 'id': call_id, 'type': 'function', 'function': {
                    'name': 'shell', 'arguments': json.dumps({'command': "printf 'native tool result' > created.txt"})}}]}
                reason = 'tool_calls'
            else:
                delta, reason = {'content': '工具已完成'}, 'stop'
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            self.wfile.write(('data: ' + json.dumps({'choices': [{'delta': delta, 'finish_reason': reason}]}) +
                              '\n\ndata: [DONE]\n\n').encode())
            if refuse_final_tool and request_count == tool_count + 1:
                final_tool_sent.set()

    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    endpoint = f'http://127.0.0.1:{server.server_port}/'
    (root / 'model.json').write_text(json.dumps({'model': 'selected', 'endpoint': endpoint, 'token': 'fixture', 'outputLimit': 4096}))
    (root / 'model.json').chmod(0o600)
    workspace = root / 'workspace'
    workspace.mkdir()
    env = {k: v for k, v in os.environ.items() if k in ('PATH', 'HOME', 'TMPDIR')}
    env.update(SHIJU_CURSOR_MODEL_CONFIG=str(root / 'model.json'), CURSOR_CONFIG_DIR=str(root / 'config'), CURSOR_DATA_DIR=str(root / 'data'), AGENT_CLI_CREDENTIAL_STORE='memory', CURSOR_AGENT_CLI_AUTHLESS_MODE='true')
    with (root / 'stderr').open('w') as stderr:
        process = subprocess.Popen([str(entry), 'acp', '--model', 'selected'], cwd=root, env=env,
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
        approvals = 0
        try:
            deadline = time.monotonic() + (600 if at_step_limit else 60)
            while time.monotonic() < deadline:
                try:
                    message = incoming.get(timeout=1)
                except queue.Empty:
                    assert process.poll() is None, 'Native Cursor exited before completion'
                    continue
                if refuse_final_tool and message.get('id') == 3 and 'error' in message:
                    assert message['error']['code'] == -32603
                    break
                assert 'error' not in message, (message.get('error'), (root / 'stderr').read_text()[-3000:])
                if message.get('id') == 1 and 'result' in message:
                    send({'id': 2, 'method': 'session/new', 'params': {'cwd': str(workspace), 'mcpServers': []}})
                elif message.get('id') == 2 and 'result' in message:
                    assert message['result']['models']['currentModelId'].split('::')[0] == 'selected', message['result']['models']
                    (root / 'session-id').write_text(json.dumps(message['result']['sessionId']))
                    send({'id': 3, 'method': 'session/prompt', 'params': {'sessionId': message['result']['sessionId'],
                          'prompt': [{'type': 'text', 'text': 'Create created.txt using save-file then finish.'}]}})
                elif message.get('id') == 3 and 'result' in message:
                    assert message['result']['stopReason'] == 'end_turn'
                    if request_count == tool_count + 1:
                        session_id = json.loads((root / 'session-id').read_text())
                        send({'id': 4, 'method': 'session/load', 'params': {'sessionId': session_id, 'cwd': str(workspace), 'mcpServers': []}})
                    else:
                        completed = True
                        break
                elif message.get('id') == 4 and 'result' in message:
                    send({'id': 3, 'method': 'session/prompt', 'params': {'sessionId': json.loads((root / 'session-id').read_text()),
                          'prompt': [{'type': 'text', 'text': 'Confirm the previous tool result from history.'}]}})
                elif message.get('method') == 'session/update':
                    updates.append(message['params']['update'])
                elif message.get('method') == 'session/request_permission':
                    approvals += 1
                    option = next(o for o in message['params']['options'] if o['kind'] == ('reject_once' if deny else 'allow_once'))
                    send({'id': message['id'], 'result': {'outcome': {'outcome': 'selected', 'optionId': option['optionId']}}})
            if refuse_final_tool:
                assert not completed and request_count == tool_count + 1
                assert final_tool_sent.wait(timeout=1)
                assert sum(u.get('sessionUpdate') == 'tool_call' for u in updates) == tool_count
                print('PASS: native ACP rejects tool execution after the step limit')
                sys.exit(0)
            assert completed, 'Native Cursor did not complete before the test deadline'
            assert approvals > 0, 'Native shell did not request current client approval'
            assert not (root / 'created.txt').exists(), 'Shell used the ACP process directory'
            if deny:
                assert not (workspace / 'created.txt').exists()
            else:
                assert (workspace / 'created.txt').read_text().strip() == 'native tool result'
            assert request_count == tool_count + 2
            assistant = next(m for m in first_followup['messages'] if m['role'] == 'assistant')
            assert assistant['reasoning_content'] == 'fixture reasoning'
            results = [m for m in first_followup['messages'] if m['role'] == 'tool']
            assert len(results) == 1 and results[0]['tool_call_id'] == 'save-fixture'
            assert ('rejected' if deny else 'success') in results[0]['content']
            assert any(u.get('status') == 'completed' and u.get('toolCallId') == 'save-fixture' for u in updates)
            assert any(u.get('sessionUpdate') == 'agent_message_chunk' and u['content'].get('text') == '工具已完成' for u in updates)
            restored = latest_request['messages']
            assert any(m.get('reasoning_content') == 'fixture reasoning' for m in restored)
            assert any(m.get('tool_call_id') == 'save-fixture' and ('rejected' if deny else 'success') in m['content'] for m in restored)
            print('PASS: native ACP model selection, permission ' + ('rejection' if deny else 'approval') + ', shell execution, resumed history, reasoning and Chinese stream')
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            server.shutdown()
