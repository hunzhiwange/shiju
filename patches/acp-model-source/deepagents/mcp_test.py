"""Exercise the reviewed CLI and real RustCodeGraph with a loopback model."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
from queue import Queue
import sqlite3
import subprocess
import sys
import tempfile
from threading import Thread

ENTRY, GRAPH, WORKSPACE = [Path(arg).resolve(strict=True) for arg in sys.argv[1:]]
SERVICE = 'ruejs_graph_probe'
TARGETS = [
    ('rustcodegraph_query', {'search': 'createRueSiteApp'}, 'app/app.tsx:32'),
    ('rustcodegraph_callers', {'symbol': 'createRueSiteApp'}, 'mountRueSiteApp'),
    ('rustcodegraph_node', {'name': 'useApp'}, '_$createApp'),
]
NAMES = [f'{SERVICE}__{name}' for name, _, _ in TARGETS]
MOUNT = {'name': SERVICE, 'command': str(GRAPH),
         'args': ['serve', '--mcp', '--path', str(WORKSPACE), '--no-watch'], 'env': [],
         '_meta': {'shiju.deepagents.toolPolicy': {
             'allow': [name for name, _, _ in TARGETS] + ['rustcodegraph_status'],
             'deny': [f'mcp__{SERVICE}__rustcodegraph_status'],
         }}}


def scenario(endpoint, home, mounted, approval):
    env = {'PATH': os.environ['PATH'], 'HOME': home, 'OPENAI_API_KEY': 'fixture',
           'OPENAI_BASE_URL': endpoint, 'LANGSMITH_TRACING': 'false'}
    peer = subprocess.Popen(['node', str(ENTRY), '--model', 'openai:gpt-4o',
                             '--workspace', str(WORKSPACE)], env=env, text=True,
                            stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    queue = Queue()
    def read_updates():
        for line in peer.stdout:
            queue.put(json.loads(line))

    Thread(target=read_updates, daemon=True).start()
    events = []
    approvals = []

    def send(**body):
        peer.stdin.write(json.dumps(dict(jsonrpc='2.0', **body)) + '\n')
        peer.stdin.flush()

    def rpc(i, method, params):
        send(id=i, method=method, params=params)
        while True:
            message = queue.get(timeout=30)
            if message.get('method') == 'session/request_permission':
                approvals.append(message['params']['toolCall'])
                send(id=message['id'], result={'outcome': {
                    'outcome': 'selected', 'optionId': approval}})
            elif message.get('method') == 'session/update':
                events.append(message['params']['update'])
            elif message.get('id') == i:
                return message

    try:
        rpc(1, 'initialize', {'protocolVersion': 1,
                             'clientCapabilities': {'fs': {'readTextFile': True, 'writeTextFile': True}}})
        settings = {'cwd': str(WORKSPACE), 'mcpServers': [MOUNT] if mounted else []}
        new = rpc(2, 'session/new', settings)
        assert 'result' in new, new
        session = {'sessionId': new['result']['sessionId']}
        assert 'result' in rpc(3, 'session/load', {**session, **settings})
        assert 'error' in rpc(4, 'session/load', {
            **session, **settings, 'mcpServers': [] if mounted else [MOUNT]})
        end = rpc(5, 'session/prompt', {
            **session, 'prompt': [{'type': 'text', 'text': 'Run the selected tools'}]})
        assert end.get('result', {}).get('stopReason') == 'end_turn', end
        starts = [event for event in events if event['sessionUpdate'] == 'tool_call']
        expected_ids = [f'call_{i}' for i in range(3)] if mounted else []
        assert [event['toolCallId'] for event in starts] == expected_ids, starts
        assert [tool['toolCallId'] for tool in approvals] == expected_ids, approvals
        assert [tool['rawInput'] for tool in approvals] == (
            [args for _, args, _ in TARGETS] if mounted else []), approvals
        for start, (_, args, text) in zip(starts, TARGETS):
            assert start['rawInput'] == args, start
            finish = [event for event in events if event['sessionUpdate'] == 'tool_call_update'
                      and event['toolCallId'] == start['toolCallId']
                      and event['status'] in ('completed', 'failed')]
            assert len(finish) == 1, finish
            if approval == 'allow-once':
                assert finish[0]['status'] == 'completed', finish
                result = json.loads(finish[0]['rawOutput'])
                assert result['isError'] is False
                assert any(block['type'] == 'text' and text in block['text']
                           for block in result['content']), result
            else:
                assert finish[0]['status'] == 'failed', finish
    finally:
        peer.stdin.close()
        try:
            peer.wait(timeout=5)
        except subprocess.TimeoutExpired:
            peer.kill()
            peer.wait(timeout=5)


def run(home, mounted, approval):
    requests = []

    class Model(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            request = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            requests.append(request)
            index = len(requests) - 1
            delta = {'content': 'Done'}
            if mounted and index < len(TARGETS):
                delta = {'tool_calls': [{'index': 0, 'id': f'call_{index}', 'type': 'function',
                                        'function': {'name': NAMES[index],
                                                     'arguments': json.dumps(TARGETS[index][1])}}]}
            finish = 'tool_calls' if 'tool_calls' in delta else 'stop'
            self.send_response(200)
            if not request.get('stream'):
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                message = {'role': 'assistant', **delta}
                if 'tool_calls' in delta:
                    message['tool_calls'] = [{key: value for key, value in call.items()
                                              if key != 'index'} for call in delta['tool_calls']]
                self.wfile.write(json.dumps({
                    'id': f'fixture_{index}', 'object': 'chat.completion', 'model': 'fixture',
                    'choices': [{'index': 0, 'message': message, 'finish_reason': finish}],
                }).encode())
                return
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            for payload, reason in [(delta, None), ({}, finish)]:
                chunk = {'id': f'fixture_{index}', 'object': 'chat.completion.chunk', 'model': 'fixture',
                         'choices': [{'index': 0, 'delta': payload, 'finish_reason': reason}]}
                self.wfile.write(f'data: {json.dumps(chunk)}\n\n'.encode())
            self.wfile.write(b'data: [DONE]\n\n')

    with ThreadingHTTPServer(('127.0.0.1', 0), Model) as server:
        thread = Thread(target=server.serve_forever)
        thread.start()
        try:
            scenario(f'http://127.0.0.1:{server.server_port}/v1', home, mounted, approval)
        finally:
            server.shutdown()
            thread.join()

    names = {desc['function']['name'] for desc in requests[0]['tools']}
    assert {name for name in names if name.startswith(SERVICE + '__')} == (
        set(NAMES) if mounted else set()), names
    output = [msg['content'] for msg in requests[-1]['messages'] if msg['role'] == 'tool']
    wanted = [text if approval == 'allow-once' else 'Tool execution denied' for _, _, text in TARGETS]
    assert len(output) == (len(wanted) if mounted else 0), output
    for value, text in zip(output, wanted):
        assert text in value, value


# Hold the private SQLite WAL sidecars open so the sandbox can read the index.
with tempfile.TemporaryDirectory() as home:
    db = sqlite3.connect(WORKSPACE / '.rustcodegraph/rustcodegraph.db')
    try:
        db.execute('SELECT name FROM sqlite_master LIMIT 1').fetchone()
        for mounted, approval in [(False, 'reject-once'), (True, 'allow-once'),
                                  (True, 'reject-once'), (False, 'reject-once')]:
            run(home, mounted, approval)
    finally:
        db.close()
print('DeepAgents MCP: baseline, tool results, denial, reload isolation, and removal passed')
