"""Exercise a reviewed Cortex launcher with a real, indexed RustCodeGraph server."""
import argparse
from hashlib import sha256
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import runpy
import select
import subprocess
from tempfile import TemporaryDirectory
import threading


MODEL = 'cortex-mcp-probe'
TOOLS = (
    ('query', {'search': 'createRueSiteApp'}, 'createRueSiteApp'),
    ('callers', {'symbol': 'createRueSiteApp'}, 'mountRueSiteApp'),
    ('node', {'name': 'useApp'}, '_$createApp'),
)
PREFIX = 'shiju_mcp_ruejs_graph_probe__rustcodegraph_'


def verify(launcher, graph, workspace, mounted, approve, load=False, cancel=False, restricted=False,
           native_read=None):
    requests, permissions, updates = [], [], []
    calls = [(PREFIX + tool, 'call_' + tool, args, text) for tool, args, text in TOOLS]
    if native_read:
        path, marker = native_read
        calls = [
            ('read', 'call_read', {'file_path': str(path)}, marker),
            (PREFIX + 'query', 'call_query', {'search': 'createRueSiteApp'}, 'createRueSiteApp'),
        ]
    required = {name for name, _, _, _ in calls}

    class Relay(BaseHTTPRequestHandler):
        def do_POST(self):
            assert self.path == '/v1/chat/completions'
            assert self.headers['Authorization'] == 'Bearer fixture'
            request = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            requests.append(request)
            names = {tool['function']['name'] for tool in request['tools']}
            if mounted and len(requests) <= len(calls) and required <= names:
                tool, call_id, args, _ = calls[len(requests) - 1]
                delta = {'tool_calls': [{'index': 0, 'id': call_id, 'type': 'function',
                          'function': {'name': tool, 'arguments': json.dumps(args)}}]}
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
        launch = state / 'cortex-launch.json'
        launch.write_text(json.dumps({'workspace': str(workspace), 'write_root': str(workspace),
                                      'endpoint': f'http://127.0.0.1:{relay.server_port}/v1',
                                      'token': 'fixture', 'model': MODEL}))
        launch.chmod(0o600)
        thread = threading.Thread(target=relay.serve_forever, daemon=True)
        thread.start()
        env = {'HOME': str(Path.home()), 'SHIJU_CORTEX_LAUNCH': str(launch),
               'VW_ACP_LAUNCH_REFERENCE': 'fixture'}
        with subprocess.Popen([launcher, 'acp', 'serve'], env=env, cwd=workspace,
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
                        permissions.append(tool['title'])
                        if cancel:
                            send({'method': 'session/cancel',
                                  'params': {'sessionId': message['params']['sessionId']}})
                        kind = 'allow_once' if approve else 'reject_once'
                        option = next(item['optionId'] for item in message['params']['options']
                                      if item['kind'] == kind)
                        send({'id': message['id'], 'result': {'outcome': {'outcome': 'selected',
                              'optionId': option}}})
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
                    definition['_meta'] = {'shiju.cortex.toolPolicy': {
                        'allow': [f'rustcodegraph_{tool}' for tool, _, _ in TOOLS] + ['rustcodegraph_status'],
                        'deny': ['mcp__ruejs_graph_probe__rustcodegraph_status'],
                    }}
                mount = [definition] if mounted else []
                created = rpc(2, 'session/new', {'cwd': str(workspace), 'mcpServers': [] if load else mount})
                session = created['sessionId']
                if load:
                    rpc(3, 'session/load', {'sessionId': session, 'cwd': str(workspace), 'mcpServers': mount})
                prompt = {'sessionId': session, 'prompt': [{'type': 'text', 'text': 'Inspect the configured graph.'}]}
                expected_stop = 'cancelled' if cancel else 'end_turn'
                assert rpc(4, 'session/prompt', prompt)['stopReason'] == expected_stop
                last_request = requests[-1]
                if load:
                    rpc(5, 'session/load', {'sessionId': session, 'cwd': str(workspace), 'mcpServers': []})
                    assert rpc(6, 'session/prompt', prompt)['stopReason'] == 'end_turn'
                    assert {tool['function']['name'] for tool in requests[-1]['tools']} == {'read', 'write', 'edit'}
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
    if native_read:
        assert required | {'write', 'edit'} <= names, f'Mounted model catalog missing native or MCP tools: {names}'
        assert len(requests) == len(calls) + 1, 'Missing model continuation'
        expected_mcp = [name for name, _, _, _ in calls if name.startswith(PREFIX)]
        assert [name for name in permissions if name.startswith(PREFIX)] == expected_mcp, permissions
        print(f'model directory: {", ".join(sorted(names))}')
        for request, (name, call_id, _, marker) in zip(requests[1:], calls):
            result = next((message['content'] for message in request['messages']
                           if message['role'] == 'tool' and message['tool_call_id'] == call_id), None)
            assert isinstance(result, str) and marker in result, (name, call_id, result)
            assert any(update.get('toolCallId') == call_id and update.get('status') == 'completed'
                       for update in updates), (name, call_id)
            owner = 'MCP' if name.startswith(PREFIX) else 'Cortex'
            print(f'{name} {call_id} {owner} completed, model received {marker!r}')
        return
    if not mounted:
        assert names == {'read', 'write', 'edit'} and not permissions
        return
    expected_names = [PREFIX + tool for tool, _, _ in TOOLS]
    assert set(expected_names) <= names, 'Mounted tools absent from model input'
    if restricted:
        assert names == set(expected_names) | {'read', 'write', 'edit'}, 'Unexpected mounted catalog'
    if cancel:
        assert len(requests) == 1 and permissions == expected_names[:1]
        return
    assert permissions == expected_names
    output = [message['content'] for message in last_request['messages'] if message['role'] == 'tool']
    expected_text = [text if approve else 'no approved active tool' for _, _, text in TOOLS]
    assert len(output) == len(expected_text) and all(text in result for text, result in zip(expected_text, output))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('launcher', 'graph', 'workspace'):
        parser.add_argument(name, type=lambda path: Path(path).resolve(strict=True))
    parser.add_argument('--coexistence', action='store_true', help='Run one mounted read and MCP prompt')
    args = parser.parse_args()
    sources = Path(__file__).resolve().parent
    contract_path = sources.parents[2] / 'crates/libs/acp-agent-registry/src/managed_runtime/contracts.json'
    files = json.loads(contract_path.read_text())['cortex-code']['files']
    for file, digest in files.items():
        if sha256((args.launcher.parent / file).read_bytes()).hexdigest() != digest:
            raise SystemExit(f'Unreviewed Cortex artifact: {file}')
    reviewed_launcher = sources / 'launcher.py'
    if args.launcher.name != 'cortex' or reviewed_launcher.read_bytes() != args.launcher.read_bytes():
        raise SystemExit('Expected the managed Cortex launcher with credential isolation')
    denials = runpy.run_path(str(reviewed_launcher))['CREDENTIAL_DENIALS']
    bridge = (sources / 'mcp_bridge.py').read_bytes() + f'\nMCP_SANDBOX += {denials!r}\n'.encode()
    if (args.launcher.parent / 'mcp_bridge.py').read_bytes() != bridge:
        raise SystemExit('Expected the managed Cortex bridge with credential isolation')
    if args.coexistence:
        with TemporaryDirectory(prefix='cortex-probe-', dir=args.workspace) as directory:
            path = Path(directory) / 'challenge.txt'
            marker = 'CORTEX_NATIVE_READ_' + os.urandom(12).hex()
            path.write_text(marker + '\n')
            verify(args.launcher, args.graph, args.workspace, True, True, native_read=(path, marker))
        raise SystemExit(0)
    for mounted, approve, load in ((False, False, False), (True, True, False), (True, False, False),
                                    (False, False, False), (True, True, True)):
        verify(args.launcher, args.graph, args.workspace, mounted, approve, load)
        print(f'passed mounted={mounted} approve={approve} load={load}', flush=True)
    verify(args.launcher, args.graph, args.workspace, True, True, cancel=True)
    print('passed cancellation while awaiting approval', flush=True)
    verify(args.launcher, args.graph, args.workspace, True, True, restricted=True)
    print('passed three-tool host policy', flush=True)
