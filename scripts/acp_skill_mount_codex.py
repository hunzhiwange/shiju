"""Fixed Codex adapter/engine identity for real skill acceptance."""
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
from verify_acp_model_source import ROOT, Unavailable


def locate_codex(entry, env):
    command = shutil.which('codex-acp', path=env.get('PATH'))
    if not command:
        raise Unavailable('fixed Codex adapter is not installed')
    bundle = Path(command).resolve()
    contract = json.loads((ROOT / 'crates/vw-acp/src/managed_runtime/contracts.json').read_text())['codex']
    if entry.get('underlying', {}).get('release') != contract['engine_version']:
        raise Unavailable('Codex launch manifest and reviewed engine pin differ')
    package = json.loads((bundle.parent.parent / 'package.json').read_text())
    if (package.get('name'), package.get('version')) != (contract['package'], contract['version']):
        raise Unavailable('Codex adapter package identity differs')
    if hashlib.sha256(bundle.read_bytes()).hexdigest() != contract['files']['dist/index.js']:
        raise Unavailable('Codex adapter hash differs from reviewed contract')
    node = shutil.which('node', path=env.get('PATH'))
    if not node:
        raise Unavailable('Node is unavailable')
    program = "console.log(require('node:module').createRequire(process.argv[1]).resolve('@openai/codex/bin/codex.js'))"
    result = subprocess.run([node, '-e', program, str(bundle)], env=env, capture_output=True, timeout=20)
    if result.returncode:
        raise Unavailable('adapter-resolved Codex engine resolution failed')
    engine = Path(result.stdout.decode().strip())
    package = json.loads((engine.parent.parent / 'package.json').read_text())
    if (package.get('name'), package.get('version')) != (contract['engine'], contract['engine_version']):
        raise Unavailable('adapter-resolved Codex engine package differs')
    result = subprocess.run([node, str(engine), '--version'], env=env, capture_output=True, timeout=20)
    if result.returncode or not re.search(r'(?<![\w.])' + re.escape(contract['engine_version']) + r'(?![\w.])', result.stdout.decode()):
        raise Unavailable('adapter-resolved Codex engine version differs')
    return {'command': str(Path(command).absolute()), 'args': [], 'env': {}}


def responses_message(fixture, body):
    """Translate wire envelopes only; discovery and correlated tools stay native."""
    messages = []
    for item in body.get('input', []):
        kind = item.get('type', 'message')
        if kind == 'message':
            content = item.get('content', '')
            if isinstance(content, list):
                content = '\n'.join(part.get('text', '') for part in content)
            messages.append({'role': item.get('role', 'user'), 'content': content})
        elif kind == 'function_call_output':
            messages.append({'role': 'tool', 'tool_call_id': item.get('call_id'), 'content': item.get('output', '')})
    if body.get('instructions'):
        messages.insert(0, {'role': 'developer', 'content': body['instructions']})
    if body.get('text', {}).get('format', {}).get('schema'):
        return {'role': 'assistant', 'content': '{"title":"Skill acceptance"}'}, 'stop'
    return fixture.response({'messages': messages, 'tools': body.get('tools', [])})


def responses_wire(message, model):
    import secrets
    import time
    if message.get('tool_calls'):
        call = message['tool_calls'][0]
        output = [{'type': 'function_call', 'id': 'fc_' + call['id'], 'call_id': call['id'],
                   'name': call['function']['name'], 'arguments': call['function']['arguments']}]
    else:
        output = [{'type': 'message', 'id': 'msg_' + secrets.token_hex(8), 'status': 'completed',
                   'role': 'assistant', 'content': [{'type': 'output_text', 'text': message['content'], 'annotations': []}]}]
    response = {'id': 'resp_' + secrets.token_hex(8), 'object': 'response', 'created_at': int(time.time()),
                'model': model, 'status': 'completed', 'output': output,
                'usage': {'input_tokens': 1, 'output_tokens': 1, 'total_tokens': 2}}
    events = [('response.created', {'response': dict(response, status='in_progress', output=[])})]
    for index, item in enumerate(output):
        initial = dict(item)
        if item['type'] == 'function_call':
            initial['arguments'] = ''
        else:
            initial['content'] = []
        events.append(('response.output_item.added', {'output_index': index, 'item': initial}))
        if item['type'] == 'function_call':
            events.extend([('response.function_call_arguments.delta', {'item_id': item['id'], 'output_index': index, 'delta': item['arguments']}),
                           ('response.function_call_arguments.done', {'item_id': item['id'], 'output_index': index, 'arguments': item['arguments']})])
        else:
            part = item['content'][0]
            events.extend([('response.content_part.added', {'item_id': item['id'], 'output_index': index, 'content_index': 0, 'part': dict(part, text='')}),
                           ('response.output_text.delta', {'item_id': item['id'], 'output_index': index, 'content_index': 0, 'delta': part['text']}),
                           ('response.output_text.done', {'item_id': item['id'], 'output_index': index, 'content_index': 0, 'text': part['text']}),
                           ('response.content_part.done', {'item_id': item['id'], 'output_index': index, 'content_index': 0, 'part': part})])
        events.append(('response.output_item.done', {'output_index': index, 'item': item}))
    events.append(('response.completed', {'response': response}))
    stream = ''.join('event: ' + event + '\ndata: ' + json.dumps(dict(value, type=event, sequence_number=index)) + '\n\n'
                     for index, (event, value) in enumerate(events)).encode()
    return response, stream
