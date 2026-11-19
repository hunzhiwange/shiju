"""Run the native VT Code ACP patch against real RustCodeGraph and a loopback model."""
import hmac
import json
import os
import queue
import secrets
import subprocess
import sys
import tempfile
import threading
from http.client import HTTPConnection
from time import monotonic
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

MODEL = 'local-probe'
SERVER = 'ruejs_graph_probe'
CASES = (
    ('rustcodegraph_query', {'search': 'createRueSiteApp'}, 'app/app.tsx:32'),
    ('rustcodegraph_callers', {'symbol': 'createRueSiteApp'}, 'mountRueSiteApp'),
    ('rustcodegraph_node', {'name': 'useApp'}, '_$createApp'),
)
TOOL_NAMES = {f'mcp__{SERVER}__{name}' for name, _, _ in CASES}


def stop(process):
    process.stdin.close()
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=3)


def run(case, engine, graph, workspace, home):
    mounted = case in ('allow', 'deny')
    approved_count = 3 if case == 'allow' else 0
    model_token, mcp_token = (secrets.token_urlsafe(32) for _ in range(2))
    env = {
        'HOME': str(home), 'TMPDIR': str(home), 'PATH': '/usr/bin:/bin',
        'NO_PROXY': '127.0.0.1,localhost', 'USER': 'vtcode-probe',
    }
    graph_process = subprocess.Popen(
        [graph, 'serve', '--mcp', '--path', workspace, '--no-watch'],
        cwd=workspace, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, text=True,
    ) if mounted else None
    lock = threading.Lock()
    grants, calls, model_turns, errors = [], [], [], []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def respond(self, status, result=None):
            body = json.dumps(result).encode() if result is not None else b''
            self.send_response(status)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            if self.path == '/v1/models':
                self.respond(200, {'object': 'list', 'data': [{'id': MODEL, 'object': 'model'}]})
            else:
                self.respond(405)

        def do_POST(self):
            route = self.path
            if route == '/mcp' and graph_process is None:
                return self.respond(404)
            token = mcp_token if route == '/mcp' else model_token
            authorization = self.headers.get('Authorization', '')
            if not hmac.compare_digest(authorization, f'Bearer {token}'):
                return self.respond(401)
            if int(self.headers['Content-Length']) > 4 * 1024 * 1024:
                return self.respond(413)
            payload = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            try:
                if route == '/mcp':
                    self.forward_mcp(payload)
                elif route == '/v1/chat/completions':
                    self.model(payload)
                else:
                    self.respond(404)
            except Exception as error:
                errors.append(str(error))
                self.respond(500)

        def forward_mcp(self, request):
            method = request['method']
            with lock:
                if method == 'tools/call':
                    params = request['params']
                    proposed = (params['name'], params['arguments'])
                    if not grants or grants.pop(0) != proposed:
                        return self.respond(403)
                    calls.append(params['name'])
                elif method not in {'initialize', 'notifications/initialized', 'tools/list'}:
                    return self.respond(403)
                graph_process.stdin.write(json.dumps(request) + '\n')
                graph_process.stdin.flush()
                if 'id' not in request:
                    return self.respond(202)
                result = json.loads(graph_process.stdout.readline())
                if method == 'tools/list':
                    allowed = {name for name, _, _ in CASES}
                    result['result']['tools'] = [
                        tool for tool in result['result']['tools'] if tool['name'] in allowed
                    ]
            self.respond(200, result)

        def model(self, request):
            assert request['model'] == MODEL, request['model']
            tools = {item['function']['name'] for item in request.get('tools', [])}
            offered = {name for name in tools if name.startswith('mcp__')}
            expected = TOOL_NAMES if mounted else set()
            assert offered == expected, f'MCP tool declarations: {sorted(offered)}, expected: {sorted(expected)}'
            index = len(model_turns)
            model_turns.append(request)
            if index:
                result = [msg for msg in request['messages'] if msg['role'] == 'tool'][-1]
                assert result['tool_call_id'] == f'probe_{index - 1}'
                expected = CASES[index - 1][2] if case == 'allow' else 'permission denied'
                assert expected in result['content'], result
            message = {'role': 'assistant', 'content': 'Finished' if mounted else 'MCP_NOT_AVAILABLE'}
            reason = 'stop'
            if index < (3 if case == 'allow' else 1 if case == 'deny' else 0):
                name, args, _ = CASES[index]
                message.update(content=None, tool_calls=[{
                    'id': f'probe_{index}', 'type': 'function',
                    'function': {'name': f'mcp__{SERVER}__{name}', 'arguments': json.dumps(args)},
                }])
                reason = 'tool_calls'
            assert not request.get('stream'), 'tool-capable VT Code probe expects Chat Completions JSON'
            self.respond(200, {
                'id': 'probe', 'object': 'chat.completion', 'created': 0, 'model': MODEL,
                'choices': [{'index': 0, 'message': message, 'finish_reason': reason}],
            })

    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    port = server.server_port
    threading.Thread(target=server.serve_forever, daemon=True).start()
    policy = home / 'probe.sb'
    quote = lambda path: json.dumps(str(path))
    policy.write_text(f'''(version 1)
(allow default)
(deny file-read-data (subpath "/Users"))
(allow file-read-data (subpath {quote(workspace)}) (subpath {quote(home)}) (literal {quote(engine)}))
(deny file-write*)
(allow file-write* (subpath {quote(home)}) (literal "/dev/null"))
(deny process-fork process-exec)
(allow process-exec (literal {quote(engine)}))
(deny network*)
(allow network-outbound (remote ip "localhost:{port}"))
''')
    config = home / 'vtcode.toml'
    config.write_text(f'''[agent]
provider = "local-provider"
default_model = "{MODEL}"
reasoning_effort = "none"
[[custom_providers]]
name = "local-provider"
display_name = "Loopback probe"
base_url = "http://127.0.0.1:{port}/v1"
api_key_env = "VT_FIXTURE_KEY"
model = "{MODEL}"
models = ["{MODEL}"]
api_format = "openai-chat"
supports_tools = true
''')
    env.update(VT_FIXTURE_KEY=model_token, VT_ACP_ENABLED='1', VT_ACP_ZED_ENABLED='1')
    process = None
    try:
        process = subprocess.Popen(
            ['/usr/bin/sandbox-exec', '-f', policy, engine, '--config', config, 'acp'],
            cwd=workspace, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
        )
        frames = queue.Queue()
        updates, approvals = [], []

        def read_frames():
            for line in process.stdout:
                frames.put(json.loads(line))
            frames.put(None)

        threading.Thread(target=read_frames, daemon=True).start()

        def send(**message):
            process.stdin.write(json.dumps({'jsonrpc': '2.0', **message}) + '\n')
            process.stdin.flush()

        def rpc(identifier, method, params):
            send(id=identifier, method=method, params=params)
            deadline = monotonic() + 25
            while (message := frames.get(timeout=max(0, deadline - monotonic()))) is not None:
                if message.get('method') == 'session/update':
                    updates.append(message['params']['update'])
                if message.get('method') == 'session/request_permission':
                    params = message['params']
                    tool = params['toolCall']
                    assert tool['title'] in TOOL_NAMES, tool
                    approvals.append(tool['toolCallId'])
                    kind = 'allow_once' if case == 'allow' else 'reject_once'
                    if case == 'allow':
                        with lock:
                            grants.append((tool['title'].removeprefix(f'mcp__{SERVER}__'), tool['rawInput']))
                    option = next(item for item in params['options'] if item['kind'] == kind)
                    send(id=message['id'], result={'outcome': {'outcome': 'selected', 'optionId': option['optionId']}})
                elif message.get('id') == identifier:
                    assert 'result' in message, errors or message
                    return message['result']
            raise RuntimeError(f'Native ACP closed before {method}')

        rpc(1, 'initialize', {'protocolVersion': 1, 'clientCapabilities': {}})
        servers = [{
            'name': SERVER, 'type': 'http', 'url': f'http://127.0.0.1:{port}/mcp',
            'headers': [{'name': 'Authorization', 'value': f'Bearer {mcp_token}'}],
        }] if mounted else []
        new = rpc(2, 'session/new', {'cwd': str(workspace), 'mcpServers': servers})
        result = rpc(3, 'session/prompt', {'sessionId': new['sessionId'],
                                         'prompt': [{'type': 'text', 'text': 'Run the staged probe.'}]})
        name, arguments, _ = CASES[0]
        replay = json.dumps({'jsonrpc': '2.0', 'id': 99, 'method': 'tools/call',
                             'params': {'name': name, 'arguments': arguments}})
        connection = HTTPConnection('127.0.0.1', port, timeout=5)
        try:
            for token, expected in [('invalid', 401), (mcp_token, 403)]:
                connection.request('POST', '/mcp', replay, {'Authorization': f'Bearer {token}'})
                response = connection.getresponse()
                assert response.status == (expected if mounted else 404), response.status
                response.read()
        finally:
            connection.close()
        assert not errors, errors
        assert result['stopReason'] == 'end_turn', result
        assert calls == [name for name, _, _ in CASES[:approved_count]], calls
        assert approvals == [f'probe_{i}' for i in range(3 if case == 'allow' else int(mounted))]
        for call in approvals:
            statuses = [update['status'] for update in updates if update.get('toolCallId') == call]
            assert statuses == (['pending', 'in_progress', 'completed'] if case == 'allow' else ['pending', 'failed']), statuses
        print(case, 'model requests:', len(model_turns), 'MCP calls:', calls, flush=True)
    finally:
        if process is not None:
            stop(process)
        server.shutdown()
        server.server_close()
        if graph_process is not None:
            stop(graph_process)


def main():
    if len(sys.argv) < 4 or any(case not in {'before', 'allow', 'deny', 'removed'} for case in sys.argv[4:]):
        raise SystemExit(f'usage: {sys.argv[0]} VTCODE RUSTCODEGRAPH INDEXED_RUEJS [before|allow|deny|removed ...]')
    engine, graph, workspace = (Path(arg).resolve(strict=True) for arg in sys.argv[1:4])
    cases = sys.argv[4:] or ['before', 'allow', 'deny', 'removed']
    with tempfile.TemporaryDirectory(prefix='vtcode-acp-probe-') as directory:
        for case in cases:
            home = Path(directory).resolve() / case
            home.mkdir()
            run(case, engine, graph, workspace, home)


if __name__ == '__main__':
    main()
