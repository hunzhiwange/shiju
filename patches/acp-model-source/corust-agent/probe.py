"""Test a disposable Corust install against an indexed RueJS workspace and a local model."""

import argparse
from hashlib import sha256
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import select
import subprocess
import tempfile
import threading
from time import monotonic


MODEL = 'corust-mcp-probe'
GRAPH = (
    ('rustcodegraph_query', {'search': 'createRueSiteApp'}, 'app/app.tsx:32'),
    ('rustcodegraph_callers', {'symbol': 'createRueSiteApp'}, 'mountRueSiteApp'),
    ('rustcodegraph_node', {'name': 'useApp', 'file': 'packages/runtime/src/hooks/useApp.ts'}, '_$createApp'),
)


def verify(launcher, binary, workspace, mounted, approved, cancelled=False, readonly=False, filtered=False):
    requests = []
    approvals = []
    events = []
    token = 'corust-probe'
    graph = GRAPH[:2] if filtered else GRAPH

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
            if mounted and step < len(graph):
                name, arguments, _ = graph[step]
                delta = {'tool_calls': [{
                    'index': 0, 'id': f'fixture-{step}', 'type': 'function',
                    'function': {'name': name, 'arguments': json.dumps(arguments)},
                }]}
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
            [launcher], env={'HOME': str(Path.home()), 'SHIJU_CORUST_LAUNCH': str(path)},
            cwd=workspace, stdin=subprocess.PIPE, stdout=subprocess.PIPE, bufsize=0,
        )

        def send(message):
            child.stdin.write(json.dumps({'jsonrpc': '2.0', **message}).encode() + b'\n')

        def rpc(identifier, method, params):
            send({'id': identifier, 'method': method, 'params': params})
            deadline = monotonic() + 30
            while monotonic() < deadline:
                assert select.select([child.stdout], [], [], max(0, deadline - monotonic()))[0], method
                frame = json.loads(child.stdout.readline())
                if frame.get('method') == 'session/request_permission':
                    approvals.append(frame['params']['toolCall'])
                    if cancelled:
                        send({'method': 'session/cancel', 'params': {'sessionId': frame['params']['sessionId']}})
                    send({'id': frame['id'], 'result': {'outcome': {
                        'outcome': 'selected', 'optionId': 'allow_once' if approved else 'reject_once',
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
    assert names == {item[0] for item in graph}
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
    args = parser.parse_args()
    before = sha256((args.workspace / 'app/app.tsx').read_bytes()).digest()
    for mounted, approved, cancelled, readonly, filtered in (
        (False, False, False, False, False), (True, True, False, False, False),
        (True, False, False, False, False), (True, True, True, False, False),
        (True, True, False, True, False), (True, True, False, False, True),
        (False, False, False, False, False),
    ):
        verify(args.launcher, args.binary, args.workspace, mounted, approved, cancelled, readonly, filtered)
        print(f'passed mounted={mounted} approved={approved} cancelled={cancelled} '
              f'readonly={readonly} filtered={filtered}', flush=True)
    assert before == sha256((args.workspace / 'app/app.tsx').read_bytes()).digest()
