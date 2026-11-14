#!/usr/bin/env python3
"""Run production siGit preset or session-model acceptance with synthetic local inference."""
import argparse
import http.server
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading

from acp_model_source_fixture import ModelFixture


def run_preset(binary, runtime, env):
    calls = []

    class Endpoint(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            calls.append({'model': body['model'],
                          'has_preset_rule': 'ACP_ACCEPTANCE_RULE' in json.dumps(body)})
            payload = ('data: ' + json.dumps({'choices': [{'delta': {'content': '验收回复'}}]})
                       + '\n\ndata: [DONE]\n\n').encode()
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.send_header('Content-Length', str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Endpoint)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        env.update(SHIJU_SIGIT_REAL_ENTRY=str(runtime / 'sigit'),
                   SHIJU_SIGIT_REAL_ENDPOINT=f'http://127.0.0.1:{server.server_port}')
        result = subprocess.run([str(binary),
            'acp_onboarding_sigit_real_builtin_profile_persists_and_runs',
            '--ignored', '--test-threads=1', '--nocapture'], env=env, check=False)
    finally:
        server.shutdown()
        server.server_close()
    print('Native preset observations:', json.dumps(calls), flush=True)
    if result.returncode or not calls or not all(
            c['model'] == 'preset-selected-model' and c['has_preset_rule'] for c in calls):
        raise RuntimeError('Production preset did not reach the expected native model request')


def run_session_model(binary, runtime, env, root):
    with ModelFixture() as fixture:
        challenge = root / 'challenge.txt'
        challenge.write_text(fixture.challenge)
        fixture.read_path = str(challenge)
        config = root / 'gateway'
        installed = config / 'acp-runtime/sigit-1.7.10-model-source-v1'
        shutil.copytree(runtime, installed)
        native = root / 'native-config'
        native.mkdir()
        manifest = root / 'manifest.json'
        manifest.write_text(json.dumps({
            'agent': 'sigit', 'mode': 'session-model', 'root': str(root),
            'fixture': fixture.url, 'key': fixture.key, 'control': fixture.control,
            'challenge': fixture.challenge, 'isolation_env': {'SIGIT_CONFIG_DIR': str(native)},
            'config': {'command': str(installed / 'sigit'), 'args': ['--acp'], 'env': {}},
            'upstream_protocol': 'chat'}))
        manifest.chmod(0o600)
        flow = root / 'flow.json'
        env.update(VIBEWINDOW_CONFIG_DIR=str(config), SIGIT_CONFIG_DIR=str(native),
                   VW_ACP_ACCEPTANCE_MANIFEST=str(manifest), VW_ACP_ACCEPTANCE_FLOW=str(flow))
        result = subprocess.run([str(binary), 'acp_model_source_real_cli_session_model',
            '--ignored', '--test-threads=1', '--nocapture'], env=env, check=False)
        observed = fixture.snapshot()
        print('Relay observations:', json.dumps(observed), flush=True)
        state = json.loads(flow.read_text()) if flow.exists() else {}
        print('Production lifecycle:', json.dumps(state), flush=True)
        if result.returncode or not observed['requests'] or observed['errors']:
            raise RuntimeError('Production session-model acceptance failed')
        if not all(state.get(k) is True for k in (
                'isolated', 'new', 'second_turn', 'cancel', 'restart', 'switch', 'handshake')):
            raise RuntimeError('Incomplete production lifecycle evidence')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime', type=Path, required=True)
    parser.add_argument('--test-binary', type=Path, required=True)
    parser.add_argument('--mode', choices=['preset', 'session-model'], required=True)
    args = parser.parse_args()
    runtime = args.runtime.resolve(strict=True)
    binary = args.test_binary.resolve(strict=True)
    if not (runtime / 'build-receipt.json').is_file() or not (runtime / 'sigit').is_file():
        raise RuntimeError('Explicit reviewed native installation required')
    env = dict(os.environ)
    env['RUST_MIN_STACK'] = str(max(int(env.get('RUST_MIN_STACK', '0')), 33554432))
    with tempfile.TemporaryDirectory(prefix='shiju-sigit-production-') as folder:
        if args.mode == 'preset':
            run_preset(binary, runtime, env)
        else:
            run_session_model(binary, runtime, env, Path(folder))


if __name__ == '__main__':
    main()
