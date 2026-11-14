"""Real reviewed siGit against synthetic loopback model responses; no account access."""
import http.server
import json
import os
from pathlib import Path
import queue
import subprocess
import tempfile
import threading
import unittest


class SigitAcceptance(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix='shiju-sigit-')
        self.root = Path(self.directory.name)
        self.responses = queue.Queue()
        self.incoming = queue.Queue()
        self.requests = []
        self.messages = []
        self.permissions = 'allow_once'
        self.files = 'allow'
        self.terminals = 'deny'
        self.client_files = True
        self.mcp_override = 'off'
        owner = self

        class Endpoint(http.server.BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                owner.requests.append(body)
                response = owner.responses.get(timeout=20)
                if isinstance(response, int):
                    self.send_response(response)
                    self.end_headers()
                    self.wfile.write(b'{"error":{"message":"synthetic endpoint rejection"}}')
                    return
                if isinstance(response, threading.Event):
                    response.wait(10)
                    response = {'content': 'cancelled synthetic response'}
                payload = ('data: ' + json.dumps({'choices': [{'delta': response}]})
                           + '\n\ndata: [DONE]\n\n').encode()
                self.send_response(200)
                self.send_header('Content-Type', 'text/event-stream')
                self.send_header('Content-Length', str(len(payload)))
                self.end_headers()
                try:
                    self.wfile.write(payload)
                except BrokenPipeError:
                    pass

        self.server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Endpoint)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.start()

    def start(self):
        self.incoming = queue.Queue()
        self.process = subprocess.Popen(
            [os.environ['SHIJU_SIGIT_TEST_ENTRY'], '--acp'], cwd=self.root,
            env={'HOME': str(self.root), 'PATH': '/usr/bin:/bin',
                 'SIGIT_CONFIG_DIR': str(self.root / 'state'), 'SIGIT_MCP': self.mcp_override,
                 'OPENAI_BASE_URL': f'http://127.0.0.1:{self.server.server_port}',
                 'OPENAI_API_KEY': 'synthetic-only', 'SIGIT_MODEL': 'selected-model',
                 'SIGIT_PERMISSIONS': 'allow', 'ONDE_DISABLE_PULSE': '1'},
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            text=True)
        incoming = self.incoming

        def read():
            for line in self.process.stdout:
                incoming.put(json.loads(line))

        threading.Thread(target=read, daemon=True).start()
        initialized = self.rpc(1, 'initialize', {'protocolVersion': 1,
            'clientCapabilities': {'fs': {'readTextFile': self.client_files, 'writeTextFile': self.client_files},
                                   'terminal': True}})
        self.assertEqual(initialized['result']['agentInfo']['version'], '1.7.10')

    def stop(self):
        self.process.terminate()
        self.process.wait(timeout=10)
        self.process.stdin.close()
        self.process.stdout.close()

    def tearDown(self):
        self.stop()
        self.server.shutdown()
        self.server.server_close()
        self.directory.cleanup()

    def send(self, message):
        self.process.stdin.write(json.dumps(message) + '\n')
        self.process.stdin.flush()

    def receive(self, request_id):
        while True:
            message = self.incoming.get(timeout=30)
            self.messages.append(message)
            method = message.get('method')
            if method == 'session/request_permission':
                self.send({'jsonrpc': '2.0', 'id': message['id'], 'result': {
                    'outcome': {'outcome': 'selected', 'optionId': self.permissions}}})
            elif method in ('fs/read_text_file', 'fs/write_text_file'):
                path = Path(message['params']['path'])
                self.assertTrue(path.resolve().is_relative_to(self.root.resolve()))
                if self.files == 'deny':
                    self.send({'jsonrpc': '2.0', 'id': message['id'], 'error': {
                        'code': -32000, 'message': 'synthetic host denial'}})
                else:
                    if method == 'fs/write_text_file':
                        path.write_text(message['params']['content'])
                        result = {}
                    else:
                        result = {'content': 'AUTHORIZED_HOST_CONTENT'}
                    self.send({'jsonrpc': '2.0', 'id': message['id'], 'result': result})
            elif method == 'terminal/create':
                self.send({'jsonrpc': '2.0', 'id': message['id'], 'error': {
                    'code': -32000, 'message': 'synthetic terminal denial'}})
            if message.get('id') == request_id and 'method' not in message:
                return message

    def rpc(self, request_id, method, params):
        self.send({'jsonrpc': '2.0', 'id': request_id, 'method': method, 'params': params})
        return self.receive(request_id)

    def new(self):
        self.session = self.rpc(2, 'session/new', {'cwd': str(self.root),
                                                'mcpServers': []})['result']['sessionId']

    def prompt(self, request_id=3):
        return self.rpc(request_id, 'session/prompt', {'sessionId': self.session,
            'prompt': [{'type': 'text', 'text': 'synthetic acceptance request'}]})

    def tool(self, name, arguments):
        self.responses.put({'tool_calls': [{'index': 0, 'id': 'synthetic-call',
            'function': {'name': name, 'arguments': json.dumps(arguments)}}]})
        self.responses.put({'content': 'synthetic completion'})
        result = self.prompt()
        self.assertIn('result', result)
        self.assertTrue(all(r['model'] == 'selected-model' for r in self.requests))

    def test_host_rejects_native_read_even_with_cli_allow(self):
        self.new()
        secret = self.root / 'secret'
        secret.write_text('DENIED_DISK_SENTINEL')
        self.permissions = 'reject_once'
        self.tool('read_file', {'path': str(secret)})
        self.assertNotIn('DENIED_DISK_SENTINEL', json.dumps(self.requests))
        self.assertTrue(any(m.get('method') == 'session/request_permission' for m in self.messages))

    def test_file_read_denial_never_falls_back(self):
        self.new()
        secret = self.root / 'secret'
        secret.write_text('DENIED_DISK_SENTINEL')
        self.files = 'deny'
        self.tool('read_file', {'path': str(secret)})
        self.assertNotIn('DENIED_DISK_SENTINEL', json.dumps(self.requests))
        self.assertTrue(any(m.get('method') == 'fs/read_text_file' for m in self.messages))

    def test_authorized_file_read_uses_host_content(self):
        self.new()
        secret = self.root / 'secret'
        secret.write_text('DENIED_DISK_SENTINEL')
        self.tool('read_file', {'path': str(secret)})
        self.assertIn('AUTHORIZED_HOST_CONTENT', json.dumps(self.requests))
        self.assertNotIn('DENIED_DISK_SENTINEL', json.dumps(self.requests))

    def test_file_write_denial_never_falls_back(self):
        self.new()
        self.files = 'deny'
        target = self.root / 'denied-write'
        self.tool('create_file', {'path': str(target), 'content': 'synthetic data'})
        self.assertFalse(target.exists())
        self.assertTrue(any(m.get('method') == 'fs/write_text_file' for m in self.messages))

    def test_authorized_file_write_reaches_host(self):
        self.new()
        target = self.root / 'authorized-write'
        self.tool('create_file', {'path': str(target), 'content': 'synthetic data'})
        self.assertEqual(target.read_text(), 'synthetic data')
        self.assertTrue(any(m.get('method') == 'fs/write_text_file' for m in self.messages))

    def test_terminal_denial_never_runs_local_command(self):
        self.new()
        target = self.root / 'denied-command'
        self.tool('run_command', {'command': f'touch {target}'})
        self.assertFalse(target.exists())
        self.assertTrue(any(m.get('method') == 'terminal/create' for m in self.messages))

    def test_root_escape_and_symlink_do_not_use_native_file_access(self):
        self.new()
        with tempfile.TemporaryDirectory(prefix='sigit-outside-') as outside:
            secret = Path(outside) / 'secret'
            secret.write_text('OUTSIDE_DENIED_SENTINEL')
            alias = self.root / 'outside-link'
            alias.symlink_to(secret)
            self.tool('read_file', {'path': str(alias)})
            self.assertNotIn('OUTSIDE_DENIED_SENTINEL', json.dumps(self.requests))
            self.assertFalse(any(m.get('method') == 'fs/read_text_file' for m in self.messages))

    def test_missing_client_file_capability_does_not_use_disk(self):
        self.stop()
        self.client_files = False
        self.start()
        self.new()
        secret = self.root / 'secret'
        secret.write_text('DENIED_DISK_SENTINEL')
        self.tool('read_file', {'path': str(secret)})
        self.assertNotIn('DENIED_DISK_SENTINEL', json.dumps(self.requests))

    def test_ambient_mcp_configuration_does_not_start_servers(self):
        self.stop()
        state = self.root / 'state'
        state.mkdir(exist_ok=True)
        marker = self.root / 'ambient-mcp-started'
        (state / 'mcp.toml').write_text(
            '[[server]]\nname="ambient"\ncommand="/bin/sh"\nargs=["-c", "touch '
            + str(marker) + '"]\n')
        self.mcp_override = 'on'
        self.start()
        self.new()
        self.responses.put({'content': 'synthetic response'})
        self.assertIn('result', self.prompt())
        self.assertFalse(marker.exists())

    def test_authentication_error_does_not_change_model_or_endpoint(self):
        self.new()
        self.responses.put(401)
        result = self.prompt()
        self.assertIn('synthetic endpoint rejection', json.dumps([result, self.messages]))
        self.assertEqual(len(self.requests), 1)
        self.assertEqual(self.requests[0]['model'], 'selected-model')

    def test_missing_model_endpoint_does_not_fall_back(self):
        self.new()
        self.responses.put(404)
        result = self.prompt()
        self.assertIn('synthetic endpoint rejection', json.dumps([result, self.messages]))
        self.assertEqual(len(self.requests), 1)
        self.assertEqual(self.requests[0]['model'], 'selected-model')

    def test_unknown_catalog_model_is_rejected_before_inference(self):
        self.new()
        result = self.rpc(3, 'session/set_config_option', {
            'sessionId': self.session, 'configId': 'sigit-model', 'value': 'unknown-fixture-model'})
        self.assertIn('error', result)
        self.assertFalse(self.requests)

    def test_two_turns_cancel_continue_and_restart_load(self):
        self.new()
        for request_id in (3, 4):
            self.responses.put({'content': 'synthetic response'})
            self.assertEqual(self.prompt(request_id)['result']['stopReason'], 'end_turn')
        release = threading.Event()
        self.responses.put(release)
        self.send({'jsonrpc': '2.0', 'id': 5, 'method': 'session/prompt', 'params': {
            'sessionId': self.session, 'prompt': [{'type': 'text', 'text': 'cancel fixture'}]}})
        # Wait until the native process reaches the external model boundary.
        for _ in range(200):
            if len(self.requests) >= 3:
                break
            threading.Event().wait(.01)
        self.assertGreaterEqual(len(self.requests), 3)
        self.send({'jsonrpc': '2.0', 'method': 'session/cancel', 'params': {'sessionId': self.session}})
        release.set()
        self.assertEqual(self.receive(5)['result']['stopReason'], 'cancelled')
        self.responses.put({'content': 'after cancellation'})
        self.assertEqual(self.prompt(6)['result']['stopReason'], 'end_turn')
        self.stop()
        self.start()
        loaded = self.rpc(7, 'session/load', {'sessionId': self.session,
            'cwd': str(self.root), 'mcpServers': []})
        self.assertIn('result', loaded)
        self.responses.put({'content': 'after restore'})
        self.assertEqual(self.prompt(8)['result']['stopReason'], 'end_turn')
        self.stop()
        self.start()
        resumed = self.rpc(9, 'session/resume', {'sessionId': self.session,
            'cwd': str(self.root), 'mcpServers': []})
        self.assertIn('result', resumed)
        self.responses.put({'content': 'after resume'})
        self.assertEqual(self.prompt(10)['result']['stopReason'], 'end_turn')
        self.assertTrue(all(r['model'] == 'selected-model' for r in self.requests))


if __name__ == '__main__':
    unittest.main(verbosity=2)
