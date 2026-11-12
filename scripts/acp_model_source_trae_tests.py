"""TRAE private-launch boundaries and an optional real native model-wire probe.

VW_TRAE_TEST_RUNTIME must name the installed reviewed runtime for the native probe.
No real model or platform credentials are used.
"""
import http.server
import importlib.util
import json
import os
from pathlib import Path
import queue
import socket
import subprocess
import tempfile
import threading
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'patches/acp-model-source/trae/launcher.py'
spec = importlib.util.spec_from_file_location('trae_launcher', SCRIPT)
launcher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(launcher)
REFERENCE = 'vw-session-model:' + 'a' * 64


class TraeBoundaries(unittest.TestCase):
    def test_binding_rejects_reference_permissions_and_non_loopback(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'binding.json'
            data = {'reference': REFERENCE, 'endpoint': 'http://127.0.0.1:12345',
                    'model': 'selected', 'output_limit': 2048, 'context_limit': 128000}
            path.write_text(json.dumps(data))
            path.chmod(0o600)
            self.assertEqual(launcher.binding(path, REFERENCE)[1], 12345)
            with self.assertRaises(ValueError):
                launcher.binding(path, 'wrong')
            path.chmod(0o644)
            with self.assertRaises(ValueError):
                launcher.binding(path, REFERENCE)
            path.chmod(0o600)
            for endpoint in ['https://127.0.0.1:12345', 'http://localhost:12345',
                             'http://127.0.0.1:12345/other', 'http://example.com']:
                data['endpoint'] = endpoint
                path.write_text(json.dumps(data))
                with self.assertRaises(ValueError):
                    launcher.binding(path, REFERENCE)

    @unittest.skipUnless(Path('/usr/bin/sandbox-exec').exists(), 'Darwin sandbox required')
    def test_kernel_policy_allows_only_the_bound_tcp_port(self):
        with socket.socket() as server, socket.socket() as unrelated:
            server.bind(('127.0.0.1', 0))
            server.listen()
            unrelated.bind(('127.0.0.1', 0))
            unrelated.listen()
            port = server.getsockname()[1]
            script = ('import socket,sys;'
                      'socket.create_connection((sys.argv[1],int(sys.argv[2])),1).close()')
            for host, target, expected in [('127.0.0.1', port, 0),
                                          ('127.0.0.1', unrelated.getsockname()[1], 1),
                                          ('198.51.100.1', 443, 1)]:
                result = subprocess.run(['/usr/bin/sandbox-exec', '-p', launcher.policy(port),
                                         os.sys.executable, '-c', script, host, str(target)],
                                        capture_output=True, timeout=5)
                self.assertEqual(result.returncode, expected, result.stderr.decode())

    @unittest.skipUnless(os.environ.get('VW_TRAE_TEST_RUNTIME'), 'explicit native runtime required')
    def test_real_native_loop_uses_private_model_without_platform_identity(self):
        calls = []

        class Probe(http.server.BaseHTTPRequestHandler):
            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                calls.append((self.path, self.headers.get('Authorization'), body['model']))
                self.send_response(400)
                self.end_headers()
                self.wfile.write(b'{"error":{"message":"owned model-wire probe"}}')

            def log_message(self, *args):
                pass

        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Probe)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            with tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                binding = root / 'binding.json'
                binding.write_text(json.dumps({'reference': REFERENCE,
                    'endpoint': f'http://127.0.0.1:{server.server_port}', 'model': 'selected',
                    'output_limit': 2048, 'context_limit': 128000}))
                binding.chmod(0o600)
                env = dict(os.environ, SHIJU_TRAE_MODEL_CONFIG=str(binding),
                           VW_ACP_LAUNCH_REFERENCE=REFERENCE, VW_TRAE_RELAY_TOKEN='probe')
                runtime = Path(os.environ['VW_TRAE_TEST_RUNTIME'])
                process = subprocess.Popen([str(runtime / 'traecli'), 'acp', 'serve'],
                    cwd=root, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                    stderr=subprocess.DEVNULL, text=True)
                responses = queue.Queue()

                def read():
                    for line in process.stdout:
                        responses.put(json.loads(line))

                threading.Thread(target=read, daemon=True).start()

                def rpc(identifier, method, params):
                    process.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': identifier,
                        'method': method, 'params': params}) + '\n')
                    process.stdin.flush()
                    while True:
                        response = responses.get(timeout=25)
                        if response.get('id') == identifier:
                            return response

                try:
                    self.assertIn('result', rpc(1, 'initialize',
                        {'protocolVersion': 1, 'clientCapabilities': {}}))
                    session = rpc(2, 'session/new', {'cwd': folder, 'mcpServers': []})['result']
                    self.assertEqual(session['models']['currentModelId'], 'session-selected')
                    self.assertIn('result', rpc(3, 'session/set_model', {
                        'sessionId': session['sessionId'], 'modelId': 'session-selected'}))
                    self.assertIn('${VW_TRAE_RELAY_TOKEN}', (root / 'trae_cli.yaml').read_text())
                    self.assertNotIn('probe', (root / 'trae_cli.yaml').read_text())
                    result = rpc(4, 'session/prompt', {'sessionId': session['sessionId'],
                        'prompt': [{'type': 'text', 'text': 'Reply hello without tools.'}]})
                    self.assertIn('error', result)
                    self.assertTrue(calls)
                    self.assertTrue(all(call == ('/v1/chat/completions', 'Bearer probe', 'selected')
                                        for call in calls), calls)
                    self.assertTrue((root / 'cache').is_dir())
                finally:
                    process.terminate()
                    process.wait(timeout=10)
                    process.stdin.close()
                    process.stdout.close()
        finally:
            server.shutdown()
            server.server_close()


if __name__ == '__main__':
    unittest.main()
