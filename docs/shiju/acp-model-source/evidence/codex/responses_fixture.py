"""Private upstream-only Responses probe. Not a replacement ACP agent or acceptance verdict."""
import json, secrets, threading, time, shlex, copy, os
from pathlib import Path
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

class ResponsesFixture:

    def __init__(self):
        self.key = secrets.token_hex(24)
        self.challenge = secrets.token_hex(16)
        self.control = secrets.token_hex(24)
        self.models = ("acp-fixture-primary", "acp-fixture-switched")
        self.records = []
        self.read_path = None
        self.second_read_path = None
        self.errors = 0
        self.rejections = []
        fixture = self

        class Handler(BaseHTTPRequestHandler):

            def log_message(self, *args):
                pass

            def do_GET(self):
                if self.path in ('/v1/models', '/v1/responses/models') and self.headers.get('Authorization') == 'Bearer ' + fixture.key:
                    data = json.dumps({'object': 'list', 'data': [{'id': model, 'object': 'model'} for model in fixture.models]}).encode()
                    self.send_response(200)
                    self.send_header('Content-Type', 'application/json')
                    self.send_header('Content-Length', str(len(data)))
                    self.end_headers()
                    self.wfile.write(data)
                    return
                if self.path != '/_evidence' or self.headers.get('Authorization') != 'Bearer ' + fixture.control:
                    fixture.errors += 1
                    known = ('/v1/models', '/v1/responses/models', '/v1/responses', '/_evidence')
                    fixture.rejections.append({'method': 'GET', 'route': self.path if self.path in known else 'other', 'model_auth': self.headers.get('Authorization') == 'Bearer ' + fixture.key})
                    self.send_response(404)
                    self.end_headers()
                    return
                data = json.dumps(fixture.snapshot()).encode()
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.send_header('Content-Length', str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            def do_POST(self):
                try:
                    length = int(self.headers['Content-Length'])
                    if not 0 < length <= 2000000:
                        raise ValueError('invalid fixture request length')
                    body = json.loads(self.rfile.read(length))
                    auth = self.headers.get('Authorization') == 'Bearer ' + fixture.key
                    if self.path != '/v1/responses' or not auth or body.get('model') not in ('acp-fixture-primary', 'acp-fixture-switched'):
                        fixture.rejections.append({'method':'POST', 'route':self.path if self.path in ('/v1/responses','/v1/chat/completions') else 'other', 'model_auth':auth, 'known_model':body.get('model') in fixture.models})
                        fixture.errors += 1
                        raise ValueError('route/auth')
                    inputs = body.get('input', [])
                    results = [x for x in inputs if x.get('type') == 'function_call_output' and fixture.challenge in str(x.get('output', ''))]
                    shapes = {}
                    for item in inputs:
                        kind = item.get('type', 'message')
                        if kind not in ('message','function_call','function_call_output','custom_tool_call','custom_tool_call_output','reasoning','item_reference'):
                            kind = 'other'
                        shapes.setdefault(kind, set()).update(item.keys())
                    shapes = {kind: sorted(keys) for kind, keys in shapes.items()}
                    schema = body.get('text', {}).get('format', {}).get('schema', {})
                    aux = bool(schema)
                    fixture.records.append({'route': self.path, 'model': body.get('model'), 'authenticated': auth, 'tool_results': len(results), 'input_shapes': shapes, 'input_has_ids': any('id' in item for item in inputs), 'input_id_roles': sorted({item.get('role') if item.get('role') in ('system','developer','user','assistant') else 'other' for item in inputs if item.get('id') is not None}), 'input_reference_count': sum(item.get('type') == 'item_reference' for item in inputs), 'has_previous_response': bool(body.get('previous_response_id')), 'auxiliary': aux, 'cancel_probe': 'CANCEL_PROBE' in json.dumps(inputs), 'keys': sorted(body.keys()), 'tool_types': sorted(set((t.get('type', '') for t in body.get('tools', []))))})
                    if 'CANCEL_PROBE' in json.dumps(inputs):
                        time.sleep(5)
                    if schema:
                        key = next(iter(schema.get('properties', {'title': {}})))
                        output = [{'type': 'message', 'id': 'msg_fixture', 'status': 'completed', 'role': 'assistant', 'content': [{'type': 'output_text', 'text': json.dumps({key: 'Offline fixture'}), 'annotations': []}]}]
                    elif len(results) < 2 and any((t.get('name') == 'exec_command' for t in body.get('tools', []))):
                        path = fixture.read_path if not results else fixture.second_read_path
                        output = [{'type': 'function_call', 'id': 'fc_fixture_' + str(len(results)), 'call_id': 'call_fixture_' + str(len(results)), 'name': 'exec_command', 'arguments': json.dumps({'cmd': 'cat ' + shlex.quote(path)})}]
                    else:
                        output = [{'type': 'message', 'id': 'msg_fixture', 'status': 'completed', 'role': 'assistant', 'content': [{'type': 'output_text', 'text': 'FIXTURE_MODEL_REPLY ' + fixture.challenge, 'annotations': []}]}]
                    response = {'id': 'resp_' + secrets.token_hex(8), 'object': 'response', 'created_at': int(time.time()), 'model': body.get('model'), 'status': 'completed', 'output': output, 'usage': {'input_tokens': 1, 'output_tokens': 1, 'total_tokens': 2}}
                    if body.get('stream'):
                        events = [('response.created', {'response': dict(response, status='in_progress', output=[])})]
                        for (i, item) in enumerate(output):
                            initial = dict(item)
                            if item['type'] == 'function_call':
                                initial['arguments'] = ''
                            else:
                                initial['content'] = []
                            events.append(('response.output_item.added', {'output_index': i, 'item': initial}))
                            if item['type'] == 'function_call':
                                events.extend([('response.function_call_arguments.delta', {'item_id': item['id'], 'output_index': i, 'delta': item['arguments']}), ('response.function_call_arguments.done', {'item_id': item['id'], 'output_index': i, 'arguments': item['arguments']})])
                            else:
                                part = item['content'][0]
                                events.extend([('response.content_part.added', {'item_id': item['id'], 'output_index': i, 'content_index': 0, 'part': dict(part, text='')}), ('response.output_text.delta', {'item_id': item['id'], 'output_index': i, 'content_index': 0, 'delta': part['text']}), ('response.output_text.done', {'item_id': item['id'], 'output_index': i, 'content_index': 0, 'text': part['text']}), ('response.content_part.done', {'item_id': item['id'], 'output_index': i, 'content_index': 0, 'part': part})])
                            events.append(('response.output_item.done', {'output_index': i, 'item': item}))
                        events.append(('response.completed', {'response': response}))
                        data = ''.join(('event: ' + event + '\ndata: ' + json.dumps(dict(value, type=event, sequence_number=i)) + '\n\n' for (i, (event, value)) in enumerate(events))).encode()
                        content = 'text/event-stream'
                    else:
                        data = json.dumps(response).encode()
                        content = 'application/json'
                    self.send_response(200)
                    self.send_header('Content-Type', content)
                    self.send_header('Content-Length', str(len(data)))
                    self.end_headers()
                    self.wfile.write(data)
                except (BrokenPipeError, ConnectionResetError):
                    pass
                except Exception:
                    fixture.errors += 1
                    self.send_response(400)
                    self.end_headers()
        self.server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.url = 'http://127.0.0.1:' + str(self.server.server_port)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *args):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()

    @property
    def read_path(self):
        return self._read_path

    @read_path.setter
    def read_path(self, value):
        self._read_path = value
        if value:
            second = Path(value).with_name('codex-challenge-second.txt')
            second.write_text(self.challenge)
            os.chmod(second, 0o600)
            self.second_read_path = str(second)

    def snapshot(self):
        return {'requests': copy.deepcopy(self.records), 'errors': self.errors, 'rejections': copy.deepcopy(self.rejections)}
