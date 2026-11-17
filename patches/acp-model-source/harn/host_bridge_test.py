"""Exercise the real Harn engine against an isolated provider and ACP file host."""
import http.server
import json
import os
from pathlib import Path
import queue
import shutil
import subprocess
import tempfile
import threading
import unittest


class HostBoundary(unittest.TestCase):
    def run_session(self, allow_write, read_rounds=0):
        requests = []
        class Provider(http.server.BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass
            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                requests.append(body)
                if len(requests) <= read_rounds:
                    delta = {'role': 'assistant', 'tool_calls': [{
                        'index': 0, 'id': f'read-{len(requests)}', 'type': 'function', 'function': {
                            'name': 'read_file', 'arguments': json.dumps({
                                'path': f'context-{len(requests)}.txt'})}}]}
                    finish = 'tool_calls'
                elif len(requests) == read_rounds + 1:
                    delta = {'role': 'assistant', 'tool_calls': [{
                        'index': 0, 'id': 'write-test', 'type': 'function', 'function': {
                            'name': 'write_file', 'arguments': json.dumps({
                                'path': 'result.txt', 'content': 'written by Harn tool call'})}}]}
                    finish = 'tool_calls'
                else:
                    delta, finish = {'content': 'Completed.'}, 'stop'
                chunks = [dict(id='fixture', object='chat.completion.chunk', choices=[{
                    'index': 0, 'delta': delta, 'finish_reason': None}]),
                    dict(id='fixture', object='chat.completion.chunk', choices=[{
                        'index': 0, 'delta': {}, 'finish_reason': finish}])]
                payload = (''.join('data: ' + json.dumps(c) + '\n\n' for c in chunks)
                           + 'data: [DONE]\n\n').encode()
                self.send_response(200)
                self.send_header('Content-Type', 'text/event-stream')
                self.send_header('Content-Length', str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)
        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Provider)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                runtime, workspace, state = [root / name for name in ['runtime', 'workspace', 'state']]
                for directory in [runtime, workspace, state]:
                    directory.mkdir(mode=0o700)
                shutil.copy2(os.environ['HARN_TEST_BINARY'], runtime / 'harn-native')
                for source, target in [('launcher.py', 'harn'), ('agent.harn', 'agent.harn')]:
                    shutil.copy2(Path(__file__).with_name(source), runtime / target)
                (runtime / 'harn').chmod(0o700)
                private_pipeline = state / 'agent.harn'
                shutil.copy2(runtime / 'agent.harn', private_pipeline)
                for index in range(1, read_rounds + 1):
                    (workspace / f'context-{index}.txt').write_text(f'Context {index}')
                config = root / 'providers.toml'
                config.write_text('[providers.alibaba-token-plan-cn]\n'
                                  f'base_url="http://127.0.0.1:{server.server_port}/v1"\n'
                                  'chat_endpoint="/chat/completions"\nauth_style="bearer"\n'
                                  'auth_env="SHIJU_HARN_RELAY_TOKEN"\n')
                env = {'PATH': os.environ['PATH'], 'HARN_PROVIDERS_CONFIG': str(config),
                       'SHIJU_HARN_PIPELINE': str(private_pipeline),
                       'SHIJU_HARN_RELAY_TOKEN': 'fixture-only', 'NO_PROXY': '127.0.0.1'}
                for key in ['XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME',
                            'HARN_SESSION_STORE_ROOT', 'HARN_STATE_DIR']:
                    env[key] = str(state)
                with (root / 'stderr').open('w') as stderr:
                    process = subprocess.Popen([str(runtime / 'harn')], cwd=workspace, env=env,
                                               stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                               stderr=stderr, text=True, bufsize=1)
                    messages = queue.Queue()
                    def receive():
                        for line in process.stdout:
                            messages.put(json.loads(line))
                    threading.Thread(target=receive, daemon=True).start()
                    writes = []
                    def send(frame):
                        process.stdin.write(json.dumps(frame) + '\n')
                        process.stdin.flush()
                    def call(number, method, params):
                        send(dict(jsonrpc='2.0', id=number, method=method, params=params))
                        for _ in range(4000):
                            frame = messages.get(timeout=30)
                            if 'method' in frame and 'id' in frame:
                                reply = {'jsonrpc': '2.0', 'id': frame['id']}
                                if frame['method'] == 'fs/write_text_file':
                                    writes.append(frame)
                                    path = Path(frame['params']['path'])
                                    self.assertEqual(path, workspace / 'result.txt')
                                    if allow_write:
                                        path.write_text(frame['params']['content'])
                                        reply['result'] = {}
                                    else:
                                        reply['error'] = {'code': -32000, 'message': 'Host denied write'}
                                elif frame['method'] == 'fs/read_text_file':
                                    path = Path(frame['params']['path'])
                                    self.assertEqual(path.parent, workspace)
                                    reply['result'] = {'content': path.read_text()}
                                else:
                                    reply['error'] = {'code': -32601, 'message': 'Method not found'}
                                send(reply)
                            elif 'method' not in frame and frame.get('id') == number:
                                return frame
                        self.fail('Too many native frames without response')
                    try:
                        self.assertIn('result', call(101, 'initialize', {
                            'protocolVersion': 1, 'clientCapabilities': {
                                'fs': {'readTextFile': True, 'writeTextFile': True}}}))
                        session = call(102, 'session/new', {'cwd': str(workspace), 'mcpServers': []})['result']['sessionId']
                        self.assertIn('result', call(103, 'session/set_config_option', {
                            'sessionId': session, 'configId': 'model',
                            'value': 'alibaba-token-plan-cn/deepseek-v4.1-flash-1017'}))
                        result = call(104, 'session/prompt', {'sessionId': session,
                            'prompt': [{'type': 'text', 'text': '请创建 result.txt 并简短回复。'}]})
                        self.assertTrue(writes, 'Tool call must traverse the ACP host')
                        self.assertTrue(requests)
                        self.assertTrue(all(body['model'] == 'deepseek-v4.1-flash-1017' for body in requests))
                        if allow_write:
                            self.assertEqual(result.get('result', {}).get('stopReason'), 'end_turn')
                            self.assertEqual((workspace / 'result.txt').read_text(), 'written by Harn tool call')
                        else:
                            self.assertFalse((workspace / 'result.txt').exists())
                            self.assertIn('error', result)
                    finally:
                        process.stdin.close()
                        try:
                            process.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            process.terminate()
                            process.wait(timeout=5)
                        process.stdout.close()
        finally:
            server.shutdown()
            server.server_close()

    def test_native_write_requires_host_authorization(self):
        self.run_session(False)

    def test_repository_exploration_can_exceed_forty_turns(self):
        self.run_session(True, read_rounds=42)

    def test_native_write_and_original_model_reach_success(self):
        self.run_session(True)


if __name__ == '__main__':
    unittest.main()
