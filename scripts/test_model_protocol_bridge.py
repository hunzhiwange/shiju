"""All nine non-Chat target directions against ProviderRelayBackend and real HTTP/SSE."""
import json
import os
from pathlib import Path
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import unittest

from model_protocol_service_wire import exchange, relay_host, request_body, replay_body, validate_json
from verify_model_protocol_routes import private_json


class ProtocolBridgeTests(unittest.TestCase):
    def run_route(self, target, source, truncated=False):
        calls = []
        def event(value):
            return 'data: ' + json.dumps(value) + '\n\n'
        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args): pass
            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                calls.append((self.path, dict(self.headers), body))
                auth = 'x-api-key' if target == 'messages' else ('x-goog-api-key' if target == 'gemini' else 'Authorization')
                expected = 'Bearer fixture-only-key' if target == 'responses' else 'fixture-only-key'
                if self.headers.get(auth) != expected or (target != 'gemini' and body.get('model') != 'wire-model'):
                    self.send_response(400); self.end_headers(); return
                stream = body.get('stream', False) or ':streamGenerateContent' in self.path
                serialized = json.dumps(body)
                replay = 'tool_result' in serialized or 'function_call_output' in serialized or 'functionResponse' in serialized
                tool = bool(body.get('tools')) and not replay
                structured = bool(body.get('output_config')) or body.get('text', {}).get('format', {}).get('type') == 'json_schema' or body.get('generationConfig', {}).get('responseMimeType') == 'application/json'
                answer = json.dumps({'value': 'OK'}) if structured else 'OK'
                if target == 'messages':
                    content = [{'type': 'text', 'text': answer}]
                    if tool: content.append({'type': 'tool_use', 'id': 'echo-1', 'name': 'audit_echo', 'input': {'value': 'probe'}})
                    final = {'id': 'fixture', 'type': 'message', 'role': 'assistant', 'model': 'wire-model', 'content': content, 'stop_reason': 'tool_use' if tool else 'end_turn', 'usage': {'input_tokens': 2, 'output_tokens': 3}}
                    frames = [{'type': 'message_start', 'message': dict(final, content=[], stop_reason=None, usage={'input_tokens': 2, 'output_tokens': 0})}]
                    for index, block in enumerate(content):
                        frames.append({'type': 'content_block_start', 'index': index, 'content_block': dict(block, text='') if block['type'] == 'text' else dict(block, input={})})
                        frames.append({'type': 'content_block_delta', 'index': index, 'delta': {'type': 'text_delta', 'text': block['text']} if block['type'] == 'text' else {'type': 'input_json_delta', 'partial_json': json.dumps(block['input'])}})
                        frames.append({'type': 'content_block_stop', 'index': index})
                    frames += [{'type': 'message_delta', 'delta': {'stop_reason': final['stop_reason']}, 'usage': {'output_tokens': 3}}, {'type': 'message_stop'}]
                elif target == 'responses':
                    output = [{'type': 'message', 'id': 'msg', 'role': 'assistant', 'content': [{'type': 'output_text', 'text': answer}]}]
                    if tool: output.append({'type': 'function_call', 'id': 'fc', 'call_id': 'echo-1', 'name': 'audit_echo', 'arguments': '{"value":"probe"}'})
                    final = {'id': 'fixture', 'object': 'response', 'model': 'wire-model', 'status': 'completed', 'output': output, 'usage': {'input_tokens': 2, 'output_tokens': 3, 'total_tokens': 5}}
                    frames = [{'type': 'response.created', 'response': {'model': 'wire-model', 'status': 'in_progress'}}, {'type': 'response.output_text.delta', 'delta': 'OK'}, {'type': 'response.completed', 'response': final}]
                else:
                    parts = [{'text': answer}]
                    if tool: parts.append({'functionCall': {'id': 'echo-1', 'name': 'audit_echo', 'args': {'value': 'probe'}}})
                    final = {'responseId': 'fixture', 'modelVersion': 'wire-model', 'candidates': [{'index': 0, 'content': {'role': 'model', 'parts': parts}, 'finishReason': 'STOP'}], 'usageMetadata': {'promptTokenCount': 2, 'candidatesTokenCount': 3, 'totalTokenCount': 5}}
                    frames = [dict(final, candidates=[{'index': 0, 'content': {'role': 'model', 'parts': parts}}]), dict(final, candidates=[{'index': 0, 'finishReason': 'STOP'}])]
                data = (''.join(event(v) for v in (frames[:-1] if truncated else frames)) if stream else json.dumps(final)).encode()
                self.send_response(200); self.send_header('Content-Type', 'text/event-stream' if stream else 'application/json'); self.send_header('Content-Length', str(len(data))); self.end_headers(); self.wfile.write(data)
        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
        try:
            with tempfile.TemporaryDirectory(prefix='shiju-native-bridge-') as directory:
                root = Path(directory) / 'relay'; base = 'http://127.0.0.1:%d' % server.server_port
                endpoint = base + {'messages': '/v1/messages', 'responses': '/v1/responses', 'gemini': '/v1beta/models/wire-model:generateContent'}[target]
                account = {'provider_id': 'protocol-bridge-fixture', 'adapter': '@ai-sdk/openai-compatible', 'model_key': 'selected', 'model_api_id': 'wire-model', 'endpoint': base + '/v1', 'credential': 'fixture-only-key'}
                auth = {'kind': 'bearer'} if target == 'responses' else {'kind': 'api_key_header', 'name': 'x-api-key' if target == 'messages' else 'x-goog-api-key'}
                catalog = {'tool_call': True, 'reasoning': False, 'limit': {'context': 8192, 'output': 1024}, 'protocols': [{'protocol': target, 'url': endpoint, 'deployment': 'fixture', 'authentication': auth, 'models': {'kind': 'models', 'ids': ['wire-model']}, 'evidence': [{'kind': 'configuration', 'source': 'fixture', 'reference': 'explicit native target', 'checked_at': '2026-11-12'}]}]}
                binary = Path(os.environ['CARGO_TARGET_DIR']) / 'debug/model-protocol-service-relay'
                with relay_host(binary, root, account, catalog, private_json) as (relay, token):
                    if truncated:
                        result, _ = exchange(relay, token, source, 'selected', request_body(source, 'selected'), root / 'truncated.json', private_json, streaming=True)
                        self.assertNotEqual(result.get('status'), 'passed'); self.assertTrue(calls); return
                    for streaming in (False, True):
                        result, reply = exchange(relay, token, source, 'selected', request_body(source, 'selected'), root / ('text-%s.json' % streaming), private_json, streaming=streaming)
                        self.assertEqual(result.get('http_status'), 200, result)
                        self.assertEqual((result if streaming else validate_json(source, reply))['status'], 'passed')
                    initial = request_body(source, 'selected', tools=True)
                    result, reply = exchange(relay, token, source, 'selected', initial, root / 'call.json', private_json)
                    self.assertEqual(result.get('http_status'), 200, result); self.assertEqual(validate_json(source, reply, tools=True)['status'], 'passed')
                    result, final = exchange(relay, token, source, 'selected', replay_body(source, initial, reply), root / 'result.json', private_json)
                    self.assertEqual(result.get('http_status'), 200, result); self.assertEqual(validate_json(source, final)['status'], 'passed')
                    self.assertIn('echo-1', json.dumps(calls[-1][2]))
                    result, _ = exchange(relay, token, source, 'selected', request_body(source, 'selected', tools=True), root / 'stream-tool.json', private_json, streaming=True, tools=True)
                    self.assertEqual(result.get('status'), 'passed', result); self.assertIn('echo-1', (root / 'stream-tool.json').read_text())
                    schema = {'type': 'object', 'properties': {'value': {'type': 'string'}}, 'required': ['value'], 'additionalProperties': False}
                    body = request_body(source, 'selected')
                    if source == 'chat': body['response_format'] = {'type': 'json_schema', 'json_schema': {'name': 'result', 'schema': schema, 'strict': True}}
                    elif source == 'messages': body['output_config'] = {'format': {'type': 'json_schema', 'schema': schema}}
                    elif source == 'responses': body['text'] = {'format': {'type': 'json_schema', 'name': 'result', 'schema': schema, 'strict': True}}
                    else: body['generationConfig'].update(responseMimeType='application/json', responseJsonSchema=schema)
                    result, reply = exchange(relay, token, source, 'selected', body, root / 'schema.json', private_json)
                    self.assertEqual(result.get('http_status'), 200, result)
                    self.assertEqual(validate_json(source, reply)['status'], 'passed')
                    sent = calls[-1][2]
                    observed = sent['output_config']['format']['schema'] if target == 'messages' else sent['text']['format']['schema'] if target == 'responses' else sent['generationConfig']['responseJsonSchema']
                    self.assertEqual(observed, schema)
                    self.assertTrue(all(':generateContent' in path or ':streamGenerateContent' in path for path, _, _ in calls) if target == 'gemini' else all(path.endswith('/' + target) for path, _, _ in calls))
        finally:
            server.shutdown(); server.server_close(); thread.join()


def route_test(target, source, truncated=False):
    def test(self): self.run_route(target, source, truncated)
    return test

for _target in ('messages', 'responses', 'gemini'):
    for _source in ('chat', 'messages', 'responses', 'gemini'):
        if _source != _target:
            setattr(ProtocolBridgeTests, 'test_%s_to_%s' % (_source, _target), route_test(_target, _source))
    setattr(ProtocolBridgeTests, 'test_%s_truncated_never_completes' % _target, route_test(_target, 'chat', True))
