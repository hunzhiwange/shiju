"""Synthetic real-CLI security regression; never logs payloads or credentials."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import tempfile
import threading
import time
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
observations = []

class Handler(BaseHTTPRequestHandler):

    def log_message(self, *args):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        text = json.dumps(body)
        observations.append({'selected_in_model_request': 'selected-probe' in text, 'builtin_in_model_request': 'kilo-config' in text, 'builtin_config_body_expanded': 'KILO_CONFIG_CONTENT' in text, 'skill_body_in_request': 'PRIVATE_PROBE_BODY' in text, 'tool_count': len(body.get('tools', [])), 'outside_scope_leaked': 'OUTSIDE_SELECTED_SCOPE' in text, 'native_body_result': any(m.get('role') == 'tool' and 'PRIVATE_PROBE_BODY' in str(m.get('content')) for m in body.get('messages', []))})
        payload = json.dumps({'id': 'probe', 'object': 'chat.completion', 'created': 0, 'model': 'probe', 'choices': [{'index': 0, 'message': {'role': 'assistant', 'content': 'done'}, 'finish_reason': 'stop'}], 'usage': {'prompt_tokens': 1, 'completion_tokens': 1, 'total_tokens': 2}}).encode()
        if args.escape and body.get('tools') and not any(m.get('role') == 'tool' for m in body.get('messages', [])):
            payload = json.dumps({'id': 'probe', 'object': 'chat.completion', 'created': 0, 'model': 'probe', 'choices': [{'index': 0, 'message': {'role': 'assistant', 'content': None, 'tool_calls': [{'id': 'scope_probe', 'type': 'function', 'function': {'name': 'skill', 'arguments': json.dumps({'name': 'selected-probe'})}}]}, 'finish_reason': 'tool_calls'}], 'usage': {'prompt_tokens': 1, 'completion_tokens': 1, 'total_tokens': 2}}).encode()
        content_type = 'application/json'
        if body.get('stream'):
            value = json.loads(payload)
            message = value['choices'][0]['message']
            if 'tool_calls' in message:
                message['tool_calls'] = [dict(message['tool_calls'][0], index=0)]
            value['object'] = 'chat.completion.chunk'
            value['choices'] = [{'index': 0, 'delta': message, 'finish_reason': value['choices'][0]['finish_reason']}]
            payload = ('data: ' + json.dumps(value) + '\n\ndata: [DONE]\n\n').encode()
            content_type = 'text/event-stream'
        self.send_response(200)
        self.send_header('Content-Type', content_type)
        self.send_header('Content-Length', str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)
server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
parser = argparse.ArgumentParser(description='Real fixed Kilocode ACP built-in isolation regression; only model HTTP is replaced')
parser.add_argument('binary', type=Path)
parser.add_argument('--mounted', action='store_true')
parser.add_argument('--escape', action='store_true', help='Verify untrusted markdown cannot substitute a file outside its selected directory')
args = parser.parse_args()
binary = args.binary.resolve()
report = {'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'cases': []}
for permission in (False, True):
    with tempfile.TemporaryDirectory(prefix='kilo-skill-probe-') as tmp:
        root = Path(tmp)
        project = root / 'project'
        project.mkdir()
        selected = root / 'selected' / 'selected-probe'
        selected.mkdir(parents=True)
        decoy = project / '.kilo/skills/unselected-probe'
        decoy.mkdir(parents=True)
        for p in (selected, decoy):
            (p / 'SKILL.md').write_text('---\nname: ' + p.name + '\ndescription: probe\n---\nPRIVATE_PROBE_BODY\n')
        if args.escape:
            if not args.mounted:
                raise ValueError('--escape requires --mounted')
            (selected.parent / 'outside.txt').write_text('OUTSIDE_SELECTED_SCOPE')
            (selected / 'SKILL.md').write_text((selected / 'SKILL.md').read_text() + '\n{file:../outside.txt}\n')
        env = {k: os.environ[k] for k in ('HOME', 'PATH', 'TMPDIR') if k in os.environ}
        env.update({k: str(root / k) for k in ('XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME', 'XDG_STATE_HOME')})
        env.update(KILO_DISABLE_AUTOUPDATE='1', KILO_DISABLE_MODELS_FETCH='1', KILO_DISABLE_DEFAULT_PLUGINS='1', KILO_DISABLE_EXTERNAL_SKILLS='1', KILO_DISABLE_CLAUDE_CODE_SKILLS='1', KILO_DISABLE_PROJECT_CONFIG='1')
        config = {'skills': {'paths': [str(selected.parent)]}, 'model': 'probe/model', 'enabled_providers': ['probe'], 'provider': {'probe': {'npm': '@ai-sdk/openai-compatible', 'models': {'model': {'id': 'probe', 'name': 'probe', 'tool_call': True, 'limit': {'context': 32000, 'output': 4096}}}, 'options': {'baseURL': 'http://127.0.0.1:' + str(server.server_port) + '/v1', 'apiKey': 'probe'}}}}
        if permission:
            config['permission'] = {'skill': {'*': 'deny', 'selected-probe': 'allow'}}
        env['KILO_CONFIG_CONTENT'] = json.dumps(config)
        if args.mounted:
            view = root / 'view'
            view.mkdir()
            (view / selected.name).symlink_to(selected, target_is_directory=True)
            env['SHIJU_KILO_SKILL_ROOT'] = str(view)
        proc = subprocess.Popen([str(binary), 'acp', '--pure'], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        case = {'permission_filter': permission, 'selected_advertised': False, 'unselected_advertised': False, 'session_created': False, 'body_advertised': False, 'commands': []}

        def send(i, m, p):
            proc.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': i, 'method': m, 'params': p}) + '\n')
            proc.stdin.flush()
        try:
            send(1, 'initialize', {'protocolVersion': 1, 'clientCapabilities': {}, 'clientInfo': {'name': 'probe', 'version': '1'}})
            deadline = time.monotonic() + 40
            while time.monotonic() < deadline:
                if not select.select([proc.stdout], [], [], 1)[0]:
                    continue
                line = proc.stdout.readline()
                if not line:
                    break
                msg = json.loads(line)
                if msg.get('id') == 1:
                    case['identity'] = msg.get('result', {}).get('agentInfo')
                    send(2, 'session/new', {'cwd': str(project), 'mcpServers': []})
                elif msg.get('id') == 2:
                    case['session_created'] = 'result' in msg
                    if not case['session_created']:
                        case['error_code'] = msg.get('error', {}).get('code')
                        break
                    send(3, 'session/prompt', {'sessionId': msg['result']['sessionId'], 'prompt': [{'type': 'text', 'text': 'Load selected-probe' if args.escape else '/kilo-config'}]})
                elif msg.get('id') == 3:
                    case['prompt_completed'] = 'result' in msg
                    break
                if msg.get('method') == 'session/request_permission':
                    option = next(o for o in msg['params']['options'] if o['kind'] == 'allow_once')
                    proc.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': msg['id'], 'result': {'outcome': {'outcome': 'selected', 'optionId': option['optionId']}}}) + '\n')
                    proc.stdin.flush()
                if msg.get('method') == 'session/update':
                    update = msg['params']['update']
                    if update.get('sessionUpdate') == 'available_commands_update':
                        names = [x['name'] for x in update.get('availableCommands', [])]
                        case['commands'] = names
                        case['selected_advertised'] = 'selected-probe' in names
                        case['unselected_advertised'] = 'unselected-probe' in names
                        case['body_advertised'] = 'PRIVATE_PROBE_BODY' in line
        finally:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
        case['model_observations'] = list(observations)
        observations.clear()
        report['cases'].append(case)
server.shutdown()
print(json.dumps(report, indent=2))
if args.mounted:
    assert all(not case['unselected_advertised'] and 'kilo-config' not in case['commands'] and case.get('prompt_completed') for case in report['cases']), 'excluded skill remains reachable'
    if args.escape:
        assert all(not any(o['outside_scope_leaked'] for o in case['model_observations']) and (not case['selected_advertised'] or any(o['native_body_result'] for o in case['model_observations'])) for case in report['cases']), 'untrusted skill source escaped its directory or activation was not observed'
    else:
        assert all(case['selected_advertised'] and not case['model_observations'] for case in report['cases']), 'excluded slash command reached model'
else:
    assert all(('kilo-config' in case['commands'] and any((o['builtin_config_body_expanded'] for o in case['model_observations'])) for case in report['cases'])), 'original negative regression was not reproduced'
