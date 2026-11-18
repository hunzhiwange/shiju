"""Exercise a reviewed Pi adapter with a real, indexed RustCodeGraph server."""
import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import select
import sqlite3
import subprocess
from tempfile import TemporaryDirectory
import threading


MODEL = 'pi-mcp-probe'
TOOLS = (
    ('query', {'search': 'createRueSiteApp'}, 'createRueSiteApp'),
    ('callers', {'symbol': 'createRueSiteApp'}, 'mountRueSiteApp'),
    ('node', {'name': 'useApp'}, '_$createApp'),
)
PREFIX = 'ruejs_graph_probe__rustcodegraph_'


def verify(launcher, graph, workspace, mounted, approve, load=False, cancel=False, restricted=False):
    requests, permissions, updates = [], [], []

    class Relay(BaseHTTPRequestHandler):
        def do_POST(self):
            assert self.path == '/v1/chat/completions'
            assert self.headers['Authorization'] == 'Bearer fixture'
            request = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            if request.get('tools'):
                requests.append(request)
            names = {tool['function']['name'] for tool in request.get('tools', [])}
            remaining = len(requests) <= len(TOOLS)
            if mounted and names and remaining and all(PREFIX + tool in names for tool, _, _ in TOOLS):
                tool, args, _ = TOOLS[len(requests) - 1]
                delta = {'tool_calls': [{'index': 0, 'id': 'call_' + tool, 'type': 'function',
                          'function': {'name': PREFIX + tool, 'arguments': json.dumps(args)}}]}
                finish = 'tool_calls'
            else:
                delta, finish = {'content': 'Probe completed.'}, 'stop'
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            for chunk, reason in (({'role': 'assistant', **delta}, None), ({}, finish)):
                body = {'id': 'chatcmpl-probe', 'object': 'chat.completion.chunk', 'created': 0,
                        'model': MODEL, 'choices': [{'index': 0, 'delta': chunk, 'finish_reason': reason}]}
                self.wfile.write(b'data: ' + json.dumps(body).encode() + b'\n\n')
            self.wfile.write(b'data: [DONE]\n\n')

    with TemporaryDirectory() as name, ThreadingHTTPServer(('127.0.0.1', 0), Relay) as relay:
        state = Path(name).resolve()
        native = state / 'native'
        native.mkdir()
        (native / 'models.json').write_text(json.dumps({'providers': {'fixture': {
            'baseUrl': f'http://127.0.0.1:{relay.server_port}/v1', 'api': 'openai-completions',
            'apiKey': 'VW_PI_PROBE_TOKEN', 'models': [
                {'id': MODEL, 'name': MODEL, 'reasoning': False, 'input': ['text'],
                 'contextWindow': 16000, 'maxTokens': 1000}]} }}))
        (native / 'settings.json').write_text(json.dumps({
            'defaultProvider': 'fixture', 'defaultModel': MODEL, 'quietStartup': True}))
        thread = threading.Thread(target=relay.serve_forever, daemon=True)
        thread.start()
        engine = launcher.parents[2] / '@mariozechner/pi-coding-agent/dist/cli.js'
        env = {'HOME': str(state), 'PATH': os.environ['PATH'], 'PI_CODING_AGENT_DIR': str(native),
               'PI_ACP_STATE_DIR': str(state / 'acp'), 'SHIJU_PI_SKILL_ROOT': '', 'PI_OFFLINE': '1',
               'VW_PI_PROBE_TOKEN': 'fixture', 'PI_ACP_PI_COMMAND': str(engine)}
        with subprocess.Popen([launcher], env=env, cwd=workspace,
                              stdin=subprocess.PIPE, stdout=subprocess.PIPE) as child:
            pending = bytearray()

            def send(message):
                child.stdin.write(json.dumps({'jsonrpc': '2.0', **message}).encode() + b'\n')
                child.stdin.flush()

            def rpc(identifier, method, params):
                send({'id': identifier, 'method': method, 'params': params})
                while True:
                    if b'\n' not in pending:
                        assert select.select([child.stdout], [], [], 25)[0], method
                        data = os.read(child.stdout.fileno(), 65536)
                        assert data, method
                        pending.extend(data)
                        continue
                    end = pending.index(b'\n')
                    message = json.loads(pending[:end])
                    del pending[:end + 1]
                    if message.get('method') == 'session/request_permission':
                        tool = message['params']['toolCall']
                        permissions.append((tool['toolCallId'], tool['title'], tool['rawInput']))
                        if cancel:
                            send({'method': 'session/cancel',
                                  'params': {'sessionId': message['params']['sessionId']}})
                            continue
                        send({'id': message['id'], 'result': {'outcome': {'outcome': 'selected',
                              'optionId': 'allow' if approve else 'reject'}}})
                    elif message.get('method') == 'session/update':
                        updates.append(message['params']['update'])
                    elif 'method' not in message and message.get('id') == identifier:
                        assert 'error' not in message, message.get('error')
                        return message['result']

            try:
                rpc(1, 'initialize', {'protocolVersion': 1, 'clientCapabilities': {}})
                definition = {'name': 'ruejs_graph_probe', 'command': str(graph), 'env': [],
                              'args': ['serve', '--mcp', '--path', str(workspace), '--no-watch']}
                if restricted:
                    definition['_meta'] = {'shiju.pi.toolPolicy': {
                        'allow': [f'rustcodegraph_{tool}' for tool, _, _ in TOOLS] + ['rustcodegraph_status'],
                        'deny': ['mcp__ruejs_graph_probe__rustcodegraph_status'],
                    }}
                mount = [definition] if mounted else []
                created = rpc(2, 'session/new', {'cwd': str(workspace), 'mcpServers': [] if load else mount})
                session = created['sessionId']
                if load:
                    assert rpc(3, 'session/prompt', {
                        'sessionId': session, 'prompt': [{'type': 'text', 'text': 'Reply briefly.'}],
                    })['stopReason'] == 'end_turn'
                    requests.clear()
                    rpc(3, 'session/load', {'sessionId': session, 'cwd': str(workspace), 'mcpServers': mount})
                prompt = {'sessionId': session, 'prompt': [{'type': 'text', 'text': 'Inspect the configured graph.'}]}
                expected_stop = 'cancelled' if cancel else 'end_turn'
                assert rpc(4, 'session/prompt', prompt)['stopReason'] == expected_stop
                first_request, last_request = requests[0], requests[-1]
                if load:
                    requests.clear()
                    rpc(5, 'session/load', {'sessionId': session, 'cwd': str(workspace), 'mcpServers': []})
                    assert rpc(6, 'session/prompt', prompt)['stopReason'] == 'end_turn'
                    assert {tool['function']['name'] for tool in requests[-1]['tools']} == {'read', 'write', 'edit', 'bash'}
            finally:
                child.stdin.close()
                try:
                    assert child.wait(timeout=8) == 0
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
                    raise
                finally:
                    relay.shutdown()
                    thread.join()

    names = {tool['function']['name'] for tool in first_request['tools']}
    if not mounted:
        assert names == {'read', 'write', 'edit', 'bash'} and not permissions
        return
    expected_names = [PREFIX + tool for tool, _, _ in TOOLS]
    assert set(expected_names) <= names, 'Mounted tools absent from model input'
    if restricted:
        assert {name for name in names if name.startswith(PREFIX)} == set(expected_names), 'Host tool policy leaked an unselected tool'
    expected_permissions = [('call_' + tool, PREFIX + tool, args) for tool, args, _ in TOOLS]
    if cancel:
        assert len(requests) == 1 and permissions == expected_permissions[:1]
        return
    assert permissions == expected_permissions
    status = 'completed' if approve else 'failed'
    results = {update['toolCallId']: update for update in updates
               if update.get('sessionUpdate') == 'tool_call_update' and update.get('status') == status}
    for call_id, _, _ in expected_permissions:
        assert results[call_id].get('rawOutput'), call_id
    outputs = [message for message in last_request['messages'] if message['role'] == 'tool']
    assert [message['tool_call_id'] for message in outputs] == [item[0] for item in expected_permissions]
    output = [message['content'] for message in outputs]
    expected_text = [text if approve else 'MCP operation denied or cancelled' for _, _, text in TOOLS]
    assert len(output) == len(expected_text) and all(text in result for text, result in zip(expected_text, output))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('launcher', 'graph', 'workspace'):
        parser.add_argument(name, type=lambda path: Path(path).resolve(strict=True))
    args = parser.parse_args()
    # The existing index is read-only in the MCP sandbox. Keep its WAL sidecars available.
    with sqlite3.connect((args.workspace / '.rustcodegraph/rustcodegraph.db').as_uri() + '?mode=rw', uri=True) as index:
        index.execute('SELECT name FROM sqlite_master LIMIT 1').fetchone()
        for mounted, approve, load in ((False, False, False), (True, True, False), (True, False, False),
                                        (False, False, False), (True, True, True)):
            verify(args.launcher, args.graph, args.workspace, mounted, approve, load)
            print(f'passed mounted={mounted} approve={approve} load={load}', flush=True)
        verify(args.launcher, args.graph, args.workspace, True, True, cancel=True)
        print('passed cancellation while awaiting approval', flush=True)
        verify(args.launcher, args.graph, args.workspace, True, True, restricted=True)
        print('passed three-tool host policy', flush=True)
