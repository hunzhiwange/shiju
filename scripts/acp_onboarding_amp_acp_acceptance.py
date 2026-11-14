#!/usr/bin/env python3
"""Run production Amp preset or session-model acceptance with synthetic local inference."""
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


def run_preset(runtime, env):
    calls = []

    class Endpoint(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            calls.append({'model': body['model'],
                          'has_preset_rule': 'ACP_ACCEPTANCE_RULE' in json.dumps(body), 'authenticated': self.headers.get('Authorization') == 'Bearer fixture-model-token' or self.headers.get('x-api-key') == 'fixture-model-token'})
            tool_results = [message for message in body.get('messages', []) if message.get('role') == 'tool']
            calls[-1]['host_write'] = any('File written by host' in json.dumps(message) for message in tool_results)
            calls[-1]['host_denied'] = any('Amp host tool rejected' in json.dumps(message) for message in tool_results)
            if self.path == '/v1/chat/completions':
                if body.get('stream'):
                    tool = next((tool.get('function', {}) for tool in body.get('tools', []) if tool.get('function', {}).get('name') == 'create_file'), None)
                    if tool and not tool_results:
                        arguments = json.dumps({'path':'amp-preset-written.txt','content':'preset-host-write'})
                        chunks = [{'choices':[{'delta':{'role':'assistant','tool_calls':[{'index':0,'id':'preset-write','type':'function','function':{'name':'create_file','arguments':arguments}}]}}]}, {'choices':[{'delta':{},'finish_reason':'tool_calls'}]}]
                    else:
                        chunks = [{'choices':[{'delta':{'role':'assistant','content':''}}]}, {'choices':[{'delta':{'content':'验收回复'}}]}, {'choices':[{'delta':{},'finish_reason':'stop'}]}]
                    payload = (''.join('data: '+json.dumps(chunk)+'\n\n' for chunk in chunks)+'data: [DONE]\n\n').encode()
                else:
                    payload = json.dumps({'id':'title','object':'chat.completion','choices':[{'index':0,'message':{'role':'assistant','content':'Acceptance'},'finish_reason':'stop'}],'usage':{'prompt_tokens':1,'completion_tokens':1,'total_tokens':2}}).encode()
                self.send_response(200)
                self.send_header('Content-Type', 'text/event-stream' if body.get('stream') else 'application/json')
                self.send_header('Content-Length', str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)
                return
            if not body.get('stream'):
                value = {'id':'title','type':'message','role':'assistant','model':body['model'],
                         'content':[{'type':'tool_use','id':'title','name':'set_title','input':{'title':'Acceptance'}}],
                         'stop_reason':'tool_use','usage':{'input_tokens':1,'output_tokens':1}}
                payload = json.dumps(value).encode()
            else:
                events = [
                    ('message_start', {'type':'message_start','message':{'id':'preset-message','type':'message','role':'assistant','model':body['model'],'content':[],'stop_reason':None,'usage':{'input_tokens':1,'output_tokens':0}}}),
                    ('content_block_start', {'type':'content_block_start','index':0,'content_block':{'type':'text','text':''}}),
                    ('content_block_delta', {'type':'content_block_delta','index':0,'delta':{'type':'text_delta','text':'验收回复'}}),
                    ('content_block_stop', {'type':'content_block_stop','index':0}),
                    ('message_delta', {'type':'message_delta','delta':{'stop_reason':'end_turn'},'usage':{'output_tokens':1}}),
                    ('message_stop', {'type':'message_stop'})]
                payload = ''.join('event: '+name+'\ndata: '+json.dumps(value)+'\n\n' for name,value in events).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream' if body.get('stream') else 'application/json')
            self.send_header('Content-Length', str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Endpoint)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        env.update(SHIJU_AMP_REAL_ENTRY=str(runtime / 'node_modules/.bin/amp-acp'),
                   SHIJU_AMP_REAL_ENDPOINT=f'http://127.0.0.1:{server.server_port}/')
        result = subprocess.run(['python3', 'scripts/cargo_slot.py', 'cargo', '--config', 'profile.test.package.vw-agent.opt-level=0', 'test', '-p', 'vw-agent', '--lib',
            'acp_onboarding_amp_acp', '--', '--include-ignored', '--test-threads=1', '--nocapture'], env=env, check=False)
    finally:
        server.shutdown()
        server.server_close()
    print('Native preset observations:', json.dumps(calls), flush=True)
    if result.returncode or not any(c.get('host_write') for c in calls) or not any(c.get('host_denied') for c in calls) or not any(c['has_preset_rule'] for c in calls) or not all(
            c['model'] == 'original' and c['authenticated'] for c in calls):
        raise RuntimeError('Production preset did not reach the expected native model request')


def run_session_model(binary, runtime, env, root):
    with ModelFixture() as fixture:
        challenge = root / 'challenge.txt'
        challenge.write_text(fixture.challenge)
        fixture.read_path = str(challenge)
        config = root / 'gateway'
        installed = config / 'acp-runtime/amp-acp-0.12.0-model-source-v1'
        shutil.copytree(runtime, installed, symlinks=True)
        native = root / 'native-config'
        native.mkdir()
        manifest = root / 'manifest.json'
        manifest.write_text(json.dumps({
            'agent': 'amp-acp', 'mode': 'session-model', 'root': str(root),
            'fixture': fixture.url, 'key': fixture.key, 'control': fixture.control,
            'challenge': fixture.challenge, 'isolation_env': {'AMP_ACP_STATE_DIR': str(native)},
            'config': {'command': str(installed / 'node_modules/.bin/amp-acp'), 'args': [], 'env': {}},
            'upstream_protocol': 'chat'}))
        manifest.chmod(0o600)
        flow = root / 'flow.json'
        env.update(VIBEWINDOW_CONFIG_DIR=str(config), AMP_ACP_STATE_DIR=str(native),
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
    parser.add_argument('--test-binary', type=Path)
    parser.add_argument('--mode', choices=['preset', 'session-model'], required=True)
    args = parser.parse_args()
    runtime = args.runtime.resolve(strict=True)
    if not (runtime / 'node_modules/amp-acp/dist/shiju-local.mjs').is_file():
        raise RuntimeError('Explicit reviewed native installation required')
    env = dict(os.environ)
    env['RUST_MIN_STACK'] = str(max(int(env.get('RUST_MIN_STACK', '0')), 33554432))
    with tempfile.TemporaryDirectory(prefix='shiju-amp_acp-production-') as folder:
        if args.mode == 'preset':
            run_preset(runtime, env)
        else:
            if args.test_binary is None:
                parser.error('--test-binary is required for session-model acceptance')
            run_session_model(args.test_binary.resolve(strict=True), runtime, env, Path(folder))


if __name__ == '__main__':
    main()
