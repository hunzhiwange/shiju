"""Focused live-loopback assertions against the actual production relay host."""
import json
import os
from pathlib import Path
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import unittest

from model_protocol_service_wire import exchange, relay_host, request_body, text_request, replay_body, validate_json
from verify_model_protocol_routes import private_json


class RelayFixtureTests(unittest.TestCase):
    def setUp(self):
        self.calls = []
        self.tool_index = [0]
        tool_index = self.tool_index
        calls = self.calls
        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args): pass
            def do_GET(self):
                data = b'{"data":[]}'
                self.send_response(200); self.send_header('Content-Length', str(len(data))); self.send_header('Content-Type', 'application/json'); self.end_headers(); self.wfile.write(data)
            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                calls.append(body)
                if self.headers.get('Authorization') != 'Bearer fixture-only-key' or body.get('model') != 'deepseek-v4.1-flash-1017':
                    self.send_response(400); self.end_headers(); return
                disabled = body.get('thinking', {}).get('type') == 'disabled'
                if body.get('stream'):
                    frames = [
                        {'id': 'fixture', 'object': 'chat.completion.chunk', 'model': 'deepseek-v4.1-flash', 'choices': [{'index': 0, 'delta': {'role': 'assistant', 'content': 'OK', 'reasoning_content': None}, 'finish_reason': None}]},
                        {'id': 'fixture', 'object': 'chat.completion.chunk', 'model': 'deepseek-v4.1-flash', 'choices': [{'index': 0, 'delta': {}, 'finish_reason': 'stop'}]},
                        {'id': 'fixture', 'object': 'chat.completion.chunk', 'model': 'deepseek-v4.1-flash', 'choices': [], 'usage': {'prompt_tokens': 1, 'completion_tokens': 1, 'total_tokens': 2}},
                    ]
                    data = (''.join('data: ' + json.dumps(frame) + '\n\n' for frame in frames) + 'data: [DONE]\n\n').encode()
                    self.send_response(200); self.send_header('Content-Type', 'text/event-stream'); self.send_header('Content-Length', str(len(data))); self.end_headers(); self.wfile.write(data)
                    return
                response = {'id': 'fixture', 'object': 'chat.completion', 'model': 'deepseek-v4.1-flash', 'choices': [{'index': 0, 'message': {'role': 'assistant', 'content': 'OK', 'reasoning_content': None if disabled else 'fixture reasoning'}, 'finish_reason': 'stop'}], 'usage': {'prompt_tokens': 1, 'completion_tokens': 1, 'total_tokens': 2}}
                if body.get('tools') and not any(m.get('role') == 'tool' for m in body['messages']):
                    response['choices'][0]['message'].update(content=None, reasoning_content=None, tool_calls=[{'index': tool_index[0], 'id': 'echo-1', 'type': 'function', 'function': {'name': 'audit_echo', 'arguments': '{"value":"probe"}'}}])
                    response['choices'][0]['finish_reason'] = 'tool_calls'
                data = json.dumps(response).encode()
                self.send_response(200); self.send_header('Content-Type', 'application/json'); self.send_header('Content-Length', str(len(data))); self.end_headers(); self.wfile.write(data)
        self.server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True); self.thread.start()
        self.directory = tempfile.TemporaryDirectory(prefix='shiju-relay-fixture-')
        self.root = Path(self.directory.name) / 'relay'
        self.account = {'provider_id': 'shiju', 'model_key': 'deepseek-v4.1-flash', 'model_api_id': 'deepseek-v4.1-flash-1017', 'endpoint': 'http://127.0.0.1:%d/v1' % self.server.server_port, 'credential': 'fixture-only-key'}
        self.catalog = {'reasoning': True, 'reasoning_options': [{'type': 'toggle'}], 'tool_call': True}
        binary = Path(os.environ['CARGO_TARGET_DIR']) / 'debug/model-protocol-service-relay'
        self.context = relay_host(binary, self.root, self.account, self.catalog, private_json)
        self.base, self.token = self.context.__enter__()

    def tearDown(self):
        self.context.__exit__(None, None, None)
        self.server.shutdown(); self.server.server_close(); self.thread.join()
        self.directory.cleanup()

    def run_request(self, body):
        result, reply = exchange(self.base, self.token, 'gemini', self.account['model_key'], body, self.root / 'response.json', private_json)
        if reply is not None: result.update(validate_json('gemini', reply))
        return result

    def test_explicit_zero_budget_maps_to_upstream_disabled_and_accepts_null_reasoning(self):
        body = request_body('gemini', self.account['model_key'])
        body['generationConfig']['thinkingConfig'] = {'thinkingBudget': 0, 'includeThoughts': False}
        self.assertEqual(self.run_request(body)['status'], 'passed')
        self.assertEqual(self.calls[-1]['thinking'], {'type': 'disabled'})

    def test_explicit_zero_budget_streaming_accepts_null_reasoning(self):
        body = request_body('gemini', self.account['model_key'])
        body['generationConfig']['thinkingConfig'] = {'thinkingBudget': 0, 'includeThoughts': False}
        result, _ = exchange(self.base, self.token, 'gemini', self.account['model_key'], body, self.root / 'stream.json', private_json, streaming=True)
        self.assertEqual(result['status'], 'passed')
        self.assertEqual(self.calls[-1]['thinking'], {'type': 'disabled'})

    def test_positive_budget_stays_rejected_without_an_expressible_route(self):
        body = request_body('gemini', self.account['model_key'])
        body['generationConfig']['thinkingConfig'] = {'thinkingBudget': 1024}
        self.assertEqual(self.run_request(body)['status'], 'capability_rejected')
        self.assertEqual(self.calls, [])

    def test_foreign_thought_signature_stays_rejected_before_dispatch(self):
        body = request_body('gemini', self.account['model_key'])
        body['contents'] = [{'role': 'model', 'parts': [{'text': 'fixture', 'thoughtSignature': 'foreign'}]}]
        self.assertEqual(self.run_request(body)['status'], 'capability_rejected')
        self.assertEqual(self.calls, [])

    def test_indexed_nonstream_tools_preserve_call_and_result_across_protocols(self):
        for protocol in ('messages', 'responses', 'gemini'):
            with self.subTest(protocol=protocol):
                body = text_request(protocol, self.account['model_key'], tools=True)
                result, reply = exchange(self.base, self.token, protocol, self.account['model_key'], body, self.root / (protocol + '-call.json'), private_json)
                self.assertEqual(result['http_status'], 200)
                self.assertEqual(validate_json(protocol, reply, tools=True)['status'], 'passed')
                body = replay_body(protocol, body, reply)
                result, reply = exchange(self.base, self.token, protocol, self.account['model_key'], body, self.root / (protocol + '-result.json'), private_json)
                self.assertEqual(result['http_status'], 200)
                self.assertEqual(validate_json(protocol, reply)['status'], 'passed')
                self.assertEqual(self.calls[-1]['messages'][-1]['tool_call_id'], 'echo-1')

    def test_nonstream_tool_index_must_match_array_position(self):
        for protocol in ('messages', 'responses', 'gemini'):
            for index in (-1, 1, '0'):
                with self.subTest(protocol=protocol, index=index):
                    self.tool_index[0] = index
                    body = text_request(protocol, self.account['model_key'], tools=True)
                    result, _ = exchange(self.base, self.token, protocol, self.account['model_key'], body, self.root / ('%s-%s-%s.json' % (protocol, type(index).__name__, index)), private_json)
                    self.assertEqual(result['http_status'], 502)
