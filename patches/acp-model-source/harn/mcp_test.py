"""Exercise a reviewed Harn launcher with a real, indexed RustCodeGraph server."""
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


MODEL = 'alibaba-token-plan-cn/deepseek-v4.1-flash-1017'
TOOLS = (
    ('query', {'search': 'createRueSiteApp'}, 'createRueSiteApp'),
    ('callers', {'symbol': 'createRueSiteApp'}, 'mountRueSiteApp'),
    ('node', {'file': 'packages/runtime/src/hooks/useApp.ts', 'offset': 1, 'limit': 10}, '_$createApp'),
)
PREFIX = 'shiju_mcp_ruejs_graph_probe__rustcodegraph_'


def verify(launcher, graph, workspace, mounted, approve, restore=None, cancel=False, restricted=False):
    requests, permissions, visible = [], [], []

    class Relay(BaseHTTPRequestHandler):
        def do_POST(self):
            assert self.path == '/v1/chat/completions'
            assert self.headers['Authorization'] == 'Bearer fixture'
            request = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            requests.append(request)
            names = {tool['function']['name'] for tool in request.get('tools', [])}
            remaining = len(requests) <= len(TOOLS)
            if mounted and remaining and all(PREFIX + tool in names for tool, _, _ in TOOLS):
                tool, args, _ = TOOLS[len(requests) - 1]
                deltas = [
                    {'tool_calls': [{'index': 0, 'id': 'call_' + tool, 'type': 'function',
                                     'function': {'name': PREFIX + tool, 'arguments': ''}}]},
                    {'tool_calls': [{'index': 0, 'function': {'arguments': json.dumps(args)}}]},
                ]
                finish = 'tool_calls'
            else:
                deltas, finish = [{'content': 'Probe completed.'}], 'stop'
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            deltas[0]['role'] = 'assistant'
            for chunk, reason in [*((delta, None) for delta in deltas), ({}, finish)]:
                body = {'id': 'chatcmpl-probe', 'object': 'chat.completion.chunk', 'created': 0,
                        'model': MODEL, 'choices': [{'index': 0, 'delta': chunk, 'finish_reason': reason}]}
                self.wfile.write(b'data: ' + json.dumps(body).encode() + b'\n\n')
                self.wfile.flush()
            self.wfile.write(b'data: [DONE]\n\n')

    with TemporaryDirectory() as name, ThreadingHTTPServer(('127.0.0.1', 0), Relay) as relay:
        state = Path(name).resolve()
        config = state / 'providers.toml'
        config.write_text('[providers.alibaba-token-plan-cn]\n'
                          f'base_url="http://127.0.0.1:{relay.server_port}/v1"\n'
                          'chat_endpoint="/chat/completions"\nauth_style="bearer"\n'
                          'auth_env="SHIJU_HARN_RELAY_TOKEN"\n')
        thread = threading.Thread(target=relay.serve_forever, daemon=True)
        thread.start()
        env = {'PATH': os.environ['PATH'], 'HARN_PROVIDERS_CONFIG': str(config),
               'SHIJU_HARN_RELAY_TOKEN': 'fixture', 'NO_PROXY': '127.0.0.1'}
        for key in ('XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME',
                    'HARN_SESSION_STORE_ROOT', 'HARN_STATE_DIR'):
            env[key] = str(state)
        env['SHIJU_HARN_WORKSPACE'] = str(workspace)
        with subprocess.Popen([launcher], env=env, cwd=state,
                              stdin=subprocess.PIPE, stdout=subprocess.PIPE) as child:
            pending = bytearray()

            def send(message):
                child.stdin.write(json.dumps({'jsonrpc': '2.0', **message}).encode() + b'\n')
                child.stdin.flush()

            def rpc(identifier, method, params, expected_error=None):
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
                        permissions.append(tool['title'])
                        if cancel:
                            send({'method': 'session/cancel',
                                  'params': {'sessionId': message['params']['sessionId']}})
                        send({'id': message['id'], 'result': {'outcome': {'outcome': 'selected',
                              'optionId': 'allow_once' if approve else 'reject_once'}}})
                    elif message.get('method') == 'session/update':
                        update = message['params']['update']
                        if update.get('sessionUpdate') == 'agent_message_chunk':
                            visible.append(update['content']['text'])
                    elif 'method' not in message and message.get('id') == identifier:
                        if expected_error:
                            assert expected_error in message['error']['message']
                            return None
                        if method == 'session/prompt' and mounted and not approve and 'error' in message:
                            assert 'completion_unverified' in message['error']['message'], message
                            return {'stopReason': 'denied'}
                        assert 'error' not in message, message.get('error')
                        return message['result']
                    elif 'method' in message and 'id' in message:
                        send({'id': message['id'], 'error': {'code': -32601, 'message': 'Probe only'}})

            try:
                rpc(1, 'initialize', {'protocolVersion': 1, 'clientCapabilities': {}})
                definition = {'name': 'ruejs_graph_probe', 'command': str(graph), 'env': [],
                              'args': ['serve', '--mcp', '--path', str(workspace), '--no-watch']}
                if restricted:
                    definition['_meta'] = {'shiju.harn.toolPolicy': {
                        'allow': [f'rustcodegraph_{tool}' for tool, _, _ in TOOLS] + ['rustcodegraph_status'],
                        'deny': ['mcp__ruejs_graph_probe__rustcodegraph_status'],
                    }}
                mount = [definition] if mounted else []
                rpc(8, 'session/new', {'cwd': str(state), 'mcpServers': mount},
                    expected_error='Could not configure explicit Harn MCP tools or session environment')
                created = rpc(2, 'session/new', {'cwd': str(workspace), 'mcpServers': [] if restore else mount})
                session = created['sessionId']
                if restore:
                    rpc(3, f'session/{restore}', {'sessionId': session, 'cwd': str(workspace), 'mcpServers': mount})
                rpc(7, 'session/set_config_option', {'sessionId': session, 'configId': 'model', 'value': MODEL})
                prompt = {'sessionId': session, 'prompt': [{'type': 'text', 'text': 'Inspect the configured graph.'}]}
                stop = rpc(4, 'session/prompt', prompt)['stopReason']
                if cancel:
                    assert stop == 'cancelled'
                elif approve or not mounted:
                    assert stop == 'end_turn'
                else:
                    assert stop in ('end_turn', 'denied')
                last_request = requests[-1]
                if restore:
                    rpc(5, f'session/{restore}', {'sessionId': session, 'cwd': str(workspace), 'mcpServers': []})
                    assert rpc(6, 'session/prompt', prompt)['stopReason'] == 'end_turn'
                    assert all(not tool['function']['name'].startswith(PREFIX) for tool in requests[-1]['tools'])
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

    names = {tool['function']['name'] for tool in requests[0]['tools']}
    if not mounted:
        assert {'read_file', 'write_file'} <= names and not any(name.startswith(PREFIX) for name in names)
        assert not permissions
        assert ''.join(visible).strip() == 'Probe completed.'
        return
    expected_names = [PREFIX + tool for tool, _, _ in TOOLS]
    assert set(expected_names) <= names, 'Mounted tools absent from model input'
    if restricted:
        assert {name for name in names if name.startswith(PREFIX)} == set(expected_names), 'Host tool policy leaked an unselected tool'
    if cancel:
        assert permissions == expected_names[:1]
        assert not any('createRueSiteApp  app/app.tsx:32' in str(request) for request in requests)
        return
    assert permissions == expected_names
    output = [str(message['content']) for message in last_request['messages'] if message['role'] == 'tool']
    expected_text = [text if approve else 'no approved active tool' for _, _, text in TOOLS]
    assert len(output) == len(expected_text) and all(text in result for text, result in zip(expected_text, output))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('launcher', 'graph', 'workspace'):
        parser.add_argument(name, type=lambda path: Path(path).resolve(strict=True))
    args = parser.parse_args()
    # Keep SQLite sidecars available to RustCodeGraph under the read-only sandbox.
    with sqlite3.connect(args.workspace / '.rustcodegraph/rustcodegraph.db') as graph_state:
        assert graph_state.execute('PRAGMA journal_mode=WAL').fetchone()[0] == 'wal'
        graph_state.execute('BEGIN')
        graph_state.execute('SELECT count(*) FROM sqlite_master').fetchone()
        for mounted, approve, restore in ((False, False, None), (True, True, None), (True, False, None),
                                           (False, False, None), (True, True, 'load'), (True, True, 'resume')):
            verify(args.launcher, args.graph, args.workspace, mounted, approve, restore)
            print(f'passed mounted={mounted} approve={approve} restore={restore}', flush=True)
        verify(args.launcher, args.graph, args.workspace, True, True, cancel=True)
        print('passed cancellation while awaiting approval', flush=True)
        verify(args.launcher, args.graph, args.workspace, True, True, restricted=True)
        print('passed three-tool host policy', flush=True)
