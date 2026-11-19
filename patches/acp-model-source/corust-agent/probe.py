"""Test a disposable Corust install against an indexed RueJS workspace and a local model."""

import argparse
from hashlib import sha256
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import select
import subprocess
import tempfile
import threading
from time import monotonic


MODEL = 'corust-mcp-probe'
NATIVE = {'read_file', 'write_file', 'glob', 'grep'}
GRAPH = (
    ('rustcodegraph_query', {'search': 'createRueSiteApp'}, 'app/app.tsx:32'),
    ('rustcodegraph_callers', {'symbol': 'createRueSiteApp'}, 'mountRueSiteApp'),
    ('rustcodegraph_node', {'name': 'useApp', 'file': 'packages/runtime/src/hooks/useApp.ts'}, '_$createApp'),
)


def verify(launcher, binary, workspace, mounted, approved, cancelled=False, readonly=False, filtered=False, coexist=False):
    requests = []
    approvals = []
    events = []
    token = 'corust-probe'
    graph = GRAPH[:2] if filtered else GRAPH

    if coexist:
        read = ('read_file', None, 'const app = useApp(RootApp).use(router).use(i18n)')
        batches = [(read,), (graph[0],), (read, graph[1])]
    else:
        batches = [(tool,) for tool in graph]

    class Relay(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def respond(self, content_type, body):
            self.send_response(200)
            self.send_header('Content-Type', content_type)
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            assert self.path == '/v1/onerouter/models'
            assert self.headers['Authorization'] == 'Bearer ' + token
            self.respond('application/json', json.dumps({
                'provider': 'onerouter',
                'models': [{'id': MODEL, 'internal_id': MODEL, 'display_name': MODEL}],
            }).encode())

        def do_POST(self):
            assert self.path == '/v1/onerouter/chat/completions'
            assert self.headers['Authorization'] == 'Bearer ' + token
            body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            assert body['model'] == MODEL
            step = len(requests)
            requests.append(body)
            tools = {item['function']['name']: item['function'] for item in body['tools']}
            if (mounted and step < len(batches)
                    and (not coexist or NATIVE.issubset(tools))):
                delta = {'tool_calls': []}
                for index, (name, arguments, _) in enumerate(batches[step]):
                    if name == 'read_file':
                        field, = tools[name]['parameters']['required']
                        arguments = {field: str(workspace / 'app/app.tsx')}
                    delta['tool_calls'].append({
                        'index': index, 'id': f'fixture-{step}-{index}', 'type': 'function',
                        'function': {'name': name, 'arguments': json.dumps(arguments)},
                    })
                reason = 'tool_calls'
            else:
                delta = {'content': 'fixture complete' if mounted else 'MCP_NOT_AVAILABLE'}
                reason = 'stop'
            data = ''
            for chunk, finish in (({'role': 'assistant', **delta}, None), ({}, reason)):
                data += 'data: ' + json.dumps({
                    'id': 'chatcmpl-fixture', 'object': 'chat.completion.chunk',
                    'created': 0, 'model': MODEL,
                    'choices': [{'index': 0, 'delta': chunk, 'finish_reason': finish}],
                }) + '\n\n'
            self.respond('text/event-stream', (data + 'data: [DONE]\n\n').encode())

    with tempfile.TemporaryDirectory() as directory, ThreadingHTTPServer(('127.0.0.1', 0), Relay) as relay:
        state = Path(directory).resolve()
        url = f'http://127.0.0.1:{relay.server_port}'
        path = state / 'launch.json'
        path.write_text(json.dumps({'workspace': str(workspace), 'endpoint': url + '/', 'token': token}))
        path.chmod(0o600)
        config = state / 'corust/config'
        config.mkdir(parents=True)
        (config / 'base.yaml').write_text(json.dumps({
            'version': '0.6.0', 'llm': {'model': MODEL, 'max_tokens': 4096},
            'gateway': {'url': url, 'timeout_secs': 30},
            'tools': {'max_file_size': 1048576},
            'log': {'log_dir': str(state/'logs'), 'log_level': 'off', 'display_in_frontend': False},
            'rag_config': {'url': url + '/'}, 'evaluation': {}, 'prompts': {},
            'scenario': 'general', 'code_quality_mode': 'restrict',
        }))
        (config / 'workspace-settings.yaml').write_text(json.dumps({'workspace_models': {str(workspace): MODEL}}))
        thread = threading.Thread(target=relay.serve_forever, daemon=True)
        thread.start()
        child = subprocess.Popen(
            [launcher], env={
                'HOME': str(Path.home()), 'SHIJU_CORUST_LAUNCH': str(path),
                'VW_ACP_LAUNCH_REFERENCE': 'corust-probe',
            },
            cwd=workspace, stdin=subprocess.PIPE, stdout=subprocess.PIPE, bufsize=0,
        )

        def send(message):
            child.stdin.write(json.dumps({'jsonrpc': '2.0', **message}).encode() + b'\n')

        def rpc(identifier, method, params):
            send({'id': identifier, 'method': method, 'params': params})
            deadline = monotonic() + 30
            while monotonic() < deadline:
                assert select.select([child.stdout], [], [], max(0, deadline - monotonic()))[0], method
                line = child.stdout.readline()
                assert line, f'Managed entry disconnected during {method}'
                frame = json.loads(line)
                if frame.get('method') == 'session/request_permission':
                    approvals.append(frame['params']['toolCall'])
                    if cancelled:
                        send({'method': 'session/cancel', 'params': {'sessionId': frame['params']['sessionId']}})
                    kind = 'allow_once' if approved else 'reject_once'
                    option = next(item['optionId'] for item in frame['params']['options'] if item['kind'] == kind)
                    send({'id': frame['id'], 'result': {'outcome': {
                        'outcome': 'selected', 'optionId': option,
                    }}})
                if frame.get('method') == 'session/update':
                    events.append(frame['params']['update'])
                if 'method' not in frame and frame.get('id') == identifier:
                    assert 'error' not in frame, frame.get('error')
                    return frame['result']
            raise TimeoutError(method)

        try:
            rpc(1, 'initialize', {'protocolVersion': 1, 'clientCapabilities': {}})
            definition = {'name': 'ruejs_graph_probe', 'command': str(binary), 'env': [],
                          'args': ['serve', '--mcp', '--path', str(workspace), '--no-watch']}
            if filtered:
                definition['_meta'] = {'shiju.corust.toolPolicy': {
                    'allow': [item[0] for item in GRAPH],
                    'deny': ['mcp__ruejs_graph_probe__rustcodegraph_node'],
                }}
            session = rpc(2, 'session/new', {'cwd': str(workspace), 'mcpServers': [definition] if mounted else []})
            if readonly:
                rpc(4, 'session/set_mode', {'sessionId': session['sessionId'], 'modeId': 'readonly'})
            result = rpc(3, 'session/prompt', {'sessionId': session['sessionId'],
                         'prompt': [{'type': 'text', 'text': 'Read the selected graph.'}]})
            assert result['stopReason'] == ('cancelled' if cancelled else 'end_turn')
        finally:
            child.stdin.close()
            try:
                status = child.wait(timeout=8)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()
                raise
            finally:
                relay.shutdown()
                thread.join()
        assert status == 0
        assert not list(state.glob('corust-mcp-*'))

    names = {tool['function']['name'] for tool in requests[0]['tools']}
    trace = [event for event in events if event['sessionUpdate'] in ('tool_call', 'tool_call_update')]
    if cancelled:
        assert len(requests) == 1 and len(approvals) == 1
        assert [item['status'] for item in trace] == ['in_progress', 'failed']
        return
    if not mounted:
        assert 'read_file' in names
        assert not names.intersection(item[0] for item in GRAPH)
        assert not approvals and not trace and len(requests) == 1
        return
    assert names == NATIVE | {item[0] for item in graph}, f'Unexpected mounted catalog: {sorted(names)}'
    if coexist:
        assert len(requests) == len(batches) + 1
        assert [call['title'] for call in approvals] == [graph[0][0], graph[1][0]]
        started, ended = trace[::2], trace[1::2]
        assert [item['status'] for item in started] == ['in_progress'] * 4
        assert [item['status'] for item in ended] == ['completed'] * 4
        assert [item['toolCallId'] for item in started] == [item['toolCallId'] for item in ended]
        native_title = 'Read ' + str(workspace / 'app/app.tsx')
        assert [item['title'] for item in started] == [native_title, graph[0][0], graph[1][0], native_title]
        for step, batch in enumerate(batches):
            followup = requests[step + 1]
            assert {tool['function']['name'] for tool in followup['tools']} == names
            messages = followup['messages']
            reply, *results = messages[-len(batch) - 1:]
            expected_ids = [f'fixture-{step}-{index}' for index in range(len(batch))]
            assert [item['id'] for item in reply['tool_calls']] == expected_ids
            assert [item['function']['name'] for item in reply['tool_calls']] == [tool[0] for tool in batch]
            assert [item['tool_call_id'] for item in results] == expected_ids
            for result, (name, _, expected) in zip(results, batch):
                assert expected in result['content'], (name, result)
        for call in approvals:
            call_id = call['toolCallId']
            assert any(item['toolCallId'] == call_id and item['status'] == 'completed'
                       for item in trace), call
        return
    started, ended = trace[::2], trace[1::2]
    status = 'completed' if approved and not readonly else 'failed'
    assert [item['status'] for item in ended] == [status] * len(graph)
    assert [item['title'] for item in started] == [item[0] for item in graph]
    if readonly:
        assert not approvals
    else:
        assert [item['title'] for item in approvals] == [item[0] for item in graph]
        assert [item['toolCallId'] for item in approvals] == [item['toolCallId'] for item in started]
    assert [item['toolCallId'] for item in ended] == [item['toolCallId'] for item in started]
    results = [item for item in requests[-1]['messages'] if item['role'] == 'tool']
    assert len(results) == len(graph)
    for result, (_, _, expected) in zip(results, graph):
        assert (expected if status == 'completed' else 'MCP permission denied') in result['content']


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('launcher', 'binary', 'workspace'):
        parser.add_argument(name, type=lambda value: Path(value).resolve(strict=True))
    parser.add_argument('--coexist', action='store_true', help='Run only the coexistence acceptance')
    args = parser.parse_args()
    before = sha256((args.workspace / 'app/app.tsx').read_bytes()).digest()
    verify(args.launcher, args.binary, args.workspace, True, True, coexist=True)
    print('passed native-only, MCP-only and mixed calls under one mounted session', flush=True)
    for mounted, approved, cancelled, readonly, filtered in (() if args.coexist else (
        (False, False, False, False, False), (True, True, False, False, False),
        (True, False, False, False, False), (True, True, True, False, False),
        (True, True, False, True, False), (True, True, False, False, True),
        (False, False, False, False, False),
    )):
        verify(args.launcher, args.binary, args.workspace, mounted, approved, cancelled, readonly, filtered)
        print(f'passed mounted={mounted} approved={approved} cancelled={cancelled} '
              f'readonly={readonly} filtered={filtered}', flush=True)
    assert before == sha256((args.workspace / 'app/app.tsx').read_bytes()).digest()
