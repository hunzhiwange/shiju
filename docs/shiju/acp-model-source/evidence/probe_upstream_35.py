"""Probe real Cursor/Droid authentication gates; never emulate successful login."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / 'scripts'))
from acp_model_source_fixture import ModelFixture


def probe(agent, binary):
    with tempfile.TemporaryDirectory(prefix='acp-upstream-gate-') as temporary, ModelFixture() as fixture:
        routes = []
        parent_handler = fixture.server.RequestHandlerClass
        class Handler(parent_handler):
            def do_POST(self):
                routes.append(self.path.split('?', 1)[0])
                super().do_POST()
        fixture.server.RequestHandlerClass = Handler
        root = Path(temporary)
        env = {key: os.environ[key] for key in ('PATH', 'HOME', 'TMPDIR') if key in os.environ}
        if agent == 'cursor':
            env.update(CURSOR_CONFIG_DIR=str(root / 'cursor'), NO_OPEN_BROWSER='1',
                       CURSOR_API_ENDPOINT=fixture.url, CURSOR_API_KEY='offline-invalid-platform-key')
            command = [str(binary), 'acp']
        else:
            (root / '.factory').mkdir(mode=0o700)
            config = root / '.factory/settings.json'
            config.write_text(json.dumps({'customModels': [{'model': fixture.models[0],
                'displayName': 'Offline Probe', 'baseUrl': fixture.url + '/v1',
                'apiKey': fixture.key, 'provider': 'generic-chat-completion-api',
                'maxOutputTokens': 4096}], 'model': 'custom:' + fixture.models[0],
                'cloudSessionSync': False}))
            config.chmod(0o600)
            env.update(FACTORY_HOME_OVERRIDE=str(root), FACTORY_DISABLE_KEYRING='1',
                       FACTORY_DROID_AUTO_UPDATE_ENABLED='0', DROID_AUTO_UPDATE_ENABLED='0')
            command = [str(binary), 'exec', '--output-format', 'acp',
                       '--model', 'custom:' + fixture.models[0]]
        version = subprocess.run([str(binary), '--version'], env=env, cwd=root,
                                 capture_output=True, timeout=20)
        process = subprocess.Popen(command, env=env, cwd=root, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)
        selector.register(process.stderr, selectors.EVENT_READ)
        buffer = b''
        responses = []
        auth_required = False

        def rpc(number, method, params):
            nonlocal buffer, auth_required
            process.stdin.write((json.dumps({'jsonrpc': '2.0', 'id': number,
                'method': method, 'params': params}) + '\n').encode())
            process.stdin.flush()
            deadline = time.monotonic() + 20
            while time.monotonic() < deadline:
                for key, _ in selector.select(.1):
                    data = os.read(key.fileobj.fileno(), 65536)
                    if not data:
                        selector.unregister(key.fileobj)
                        continue
                    if key.fileobj != process.stdout:
                        continue
                    buffer += data
                    while b'\n' in buffer:
                        line, buffer = buffer.split(b'\n', 1)
                        try:
                            response = json.loads(line)
                        except ValueError:
                            continue
                        if response.get('id') != number:
                            continue
                        error = response.get('error', {})
                        message = error.get('message', '').lower()
                        auth_required |= any(word in message for word in
                            ('authenticat', 'log in', 'login', 'unauthorized', 'sign in'))
                        responses.append({'method': method, 'has_result': 'result' in response,
                                          'error_code': error.get('code')})
                        return response
                if process.poll() is not None:
                    break
            responses.append({'method': method, 'timed_out': True})
            return {}
        try:
            initialized = rpc(1, 'initialize', {'protocolVersion': 1,
                'clientCapabilities': {}, 'clientInfo': {'name': 'offline-gate', 'version': '1'}})
            if 'result' in initialized:
                created = rpc(2, 'session/new', {'cwd': str(root), 'mcpServers': []})
                session = created.get('result', {}).get('sessionId')
                if session:
                    rpc(3, 'session/prompt', {'sessionId': session,
                        'prompt': [{'type': 'text', 'text': 'Reply hello.'}]})
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
            selector.close()
            for stream in (process.stdin, process.stdout, process.stderr):
                stream.close()
        snapshot = fixture.snapshot()
        return {'agent': agent, 'model_source_verified': False, 'binary_sha256':
            hashlib.sha256(binary.read_bytes()).hexdigest(), 'version_exit_code': version.returncode,
            'version': version.stdout.decode(errors='replace').strip()[:100],
            'responses': responses, 'authentication_required': auth_required,
            'http_paths': sorted(set(routes)), 'model_requests': len(snapshot['requests']), 'fixture_errors': snapshot['errors'],
            'process_exit_code': process.returncode,
            'scope': 'Real CLI gate probe; no successful platform authentication simulated; not source acceptance'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    binaries = {'cursor': ROOT / 'target/acp-model-source-tools/cursor-2026.10.22-27dcf5f/dist-package/cursor-agent',
                'droid': ROOT / 'target/acp-model-source-tools/droid/droid'}
    reports = [probe(agent, binary) for agent, binary in binaries.items()]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({'observed_on': '2026-11-05', 'probes': reports}, indent=2) + '\n')
