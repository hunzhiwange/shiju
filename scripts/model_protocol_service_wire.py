"""Wire observations through the public ACP relay; audit_echo is a fixed reply."""
from contextlib import contextmanager
import json
import os
from pathlib import Path
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def nonempty(value):
    return isinstance(value, str) and bool(value.strip())


def calls_valid(calls):
    return bool(calls) and len({call[0] for call in calls}) == len(calls) and all(
        nonempty(identity) and name == 'audit_echo' and arguments == {'value': 'probe'}
        for identity, name, arguments in calls)


def validate_json(protocol, body, tools=False, require_thinking=False):
    invalid = {'status': 'invalid_response'}
    if not isinstance(body, dict) or body.get('error'):
        return invalid
    try:
        calls, thinking = [], False
        if protocol == 'chat':
            choice = body['choices'][0]; message = choice['message']
            if message.get('role') != 'assistant': return invalid
            content = nonempty(message.get('content')); terminal = choice.get('finish_reason') == ('tool_calls' if tools else 'stop')
            model = body.get('model'); thinking = nonempty(message.get('reasoning_content'))
            for call in message.get('tool_calls') or []:
                calls.append((call.get('id'), call['function'].get('name'), json.loads(call['function']['arguments'])))
        elif protocol == 'messages':
            if body.get('type') != 'message' or body.get('role') != 'assistant': return invalid
            content = any(b.get('type') == 'text' and nonempty(b.get('text')) for b in body['content'])
            terminal = body.get('stop_reason') == ('tool_use' if tools else 'end_turn'); model = body.get('model')
            thinking = any(b.get('type') in ('thinking', 'redacted_thinking') for b in body['content'])
            calls = [(b.get('id'), b.get('name'), b.get('input')) for b in body['content'] if b.get('type') == 'tool_use']
        elif protocol == 'responses':
            terminal = body.get('status') == 'completed'; model = body.get('model')
            content = any(m.get('type') == 'message' and any(b.get('type') == 'output_text' and nonempty(b.get('text')) for b in m.get('content', [])) for m in body['output'])
            thinking = any(m.get('type') == 'reasoning' for m in body['output'])
            calls = [(m.get('call_id'), m.get('name'), json.loads(m['arguments'])) for m in body['output'] if m.get('type') == 'function_call']
        elif protocol == 'gemini':
            candidate = body['candidates'][0]; parts = candidate['content']['parts']
            terminal = candidate.get('finishReason') == 'STOP'; model = body.get('modelVersion')
            content = any(nonempty(p.get('text')) and p.get('thought') is not True for p in parts)
            thinking = any(p.get('thought') is True or 'thoughtSignature' in p for p in parts)
            calls = [(p['functionCall'].get('id'), p['functionCall'].get('name'), p['functionCall'].get('args')) for p in parts if 'functionCall' in p]
        else:
            return invalid
        if (not terminal or not nonempty(model) or (require_thinking and not thinking)
                or not (calls_valid(calls) if tools else content)):
            return invalid
        return {'status': 'passed', 'actual_model_id': model, 'terminal': True,
                'has_content': content, 'tool_calls': len(calls), 'thinking_present': thinking}
    except (KeyError, IndexError, TypeError, ValueError):
        return invalid


def validate_events(protocol, events, tools=False):
    text, terminal, finish, model = False, False, False, None
    sequence, calls = [], []
    fragments = {}
    try:
        for event, body in events:
            sequence.append(event)
            if protocol == 'chat':
                if event == 'done': terminal = True; continue
                if body.get('error'): return {'status': 'invalid_response'}
                model = body.get('model') or model
                for choice in body.get('choices', []):
                    delta = choice.get('delta', {})
                    text = text or nonempty(delta.get('content'))
                    finish = finish or choice.get('finish_reason') == ('tool_calls' if tools else 'stop')
                    for call in delta.get('tool_calls') or []:
                        item = fragments.setdefault(call['index'], {'id': '', 'name': '', 'arguments': ''})
                        for key in ('id', 'name', 'arguments'):
                            item[key] += (call.get(key, '') if key == 'id' else call.get('function', {}).get(key, ''))
            elif protocol == 'messages':
                kind = body.get('type', event)
                if kind == 'error': return {'status': 'invalid_response'}
                model = body.get('message', {}).get('model') or model
                delta = body.get('delta', {})
                text = text or kind == 'content_block_delta' and delta.get('type') == 'text_delta' and nonempty(delta.get('text'))
                finish = finish or kind == 'message_delta' and delta.get('stop_reason') == ('tool_use' if tools else 'end_turn')
                terminal = terminal or kind == 'message_stop'
                if kind == 'content_block_start' and body.get('content_block', {}).get('type') == 'tool_use':
                    block = body['content_block']; fragments[body['index']] = {'id': block['id'], 'name': block['name'], 'arguments': '', 'input': block['input']}
                if kind == 'content_block_delta' and delta.get('type') == 'input_json_delta':
                    fragments[body['index']]['arguments'] += delta['partial_json']
            elif protocol == 'responses':
                kind = body.get('type', event)
                if kind in ('error', 'response.failed', 'response.incomplete'): return {'status': 'invalid_response'}
                model = body.get('response', {}).get('model') or model
                text = text or kind == 'response.output_text.delta' and nonempty(body.get('delta'))
                terminal = terminal or kind == 'response.completed' and body.get('response', {}).get('status') == 'completed'
                finish = terminal
                if kind == 'response.completed':
                    calls = [(item['call_id'], item['name'], json.loads(item['arguments'])) for item in body.get('response', {}).get('output', []) if item.get('type') == 'function_call']
            elif protocol == 'gemini':
                if body.get('error'): return {'status': 'invalid_response'}
                model = body.get('modelVersion') or model
                for candidate in body.get('candidates', []):
                    parts = candidate.get('content', {}).get('parts', [])
                    text = text or any(nonempty(p.get('text')) and not p.get('thought') for p in parts)
                    terminal = terminal or candidate.get('finishReason') == 'STOP'
                    calls.extend((p['functionCall'].get('id'), p['functionCall'].get('name'), p['functionCall'].get('args')) for p in parts if 'functionCall' in p)
                finish = terminal
        if protocol in ('chat', 'messages'):
            calls = [(item['id'], item['name'], json.loads(item['arguments']) if item['arguments'] else item.get('input')) for item in fragments.values()]
        valid = (calls_valid(calls) if tools else text) and terminal and finish and nonempty(model)
        return {'status': 'passed' if valid else 'invalid_response', 'terminal': bool(terminal and finish),
                'actual_model_id': model, 'content_observed': bool(text), 'tool_calls': len(calls), 'event_sequence': sequence}
    except (AttributeError, TypeError, KeyError, IndexError, ValueError):
        return {'status': 'invalid_response'}


def request_body(protocol, model, tools=False):
    prompt = 'Call audit_echo with value probe. After its result reply OK.' if tools else 'Reply with OK.'
    schema = {'type': 'object', 'properties': {'value': {'type': 'string', 'enum': ['probe']}}, 'required': ['value'], 'additionalProperties': False}
    function = {'name': 'audit_echo', 'description': 'Return a fixed acceptance value.', 'parameters': schema}
    if protocol in ('chat', 'messages'):
        body = {'model': model, 'max_tokens': 1024, 'messages': [{'role': 'user', 'content': prompt}]}
        if tools: body['tools'] = [{'type': 'function', 'function': function}] if protocol == 'chat' else [{'name': 'audit_echo', 'description': function['description'], 'input_schema': schema}]
    elif protocol == 'responses':
        body = {'model': model, 'input': prompt, 'max_output_tokens': 1024, 'store': False}
        if tools: body['tools'] = [{'type': 'function', **function}]
    else:
        body = {'contents': [{'role': 'user', 'parts': [{'text': prompt}]}], 'generationConfig': {'maxOutputTokens': 1024}}
        if tools: body['tools'] = [{'functionDeclarations': [function]}]
    return body


def text_request(protocol, model, tools=False):
    body = request_body(protocol, model, tools)
    if protocol == 'gemini':
        body['generationConfig']['thinkingConfig'] = {'thinkingBudget': 0, 'includeThoughts': False}
    return body


def replay_body(protocol, initial, reply):
    body = json.loads(json.dumps(initial))
    if protocol == 'chat':
        message = reply['choices'][0]['message']; body['messages'].append(message)
        body['messages'].extend({'role': 'tool', 'tool_call_id': c['id'], 'content': '{"value":"probe"}'} for c in message['tool_calls'])
    elif protocol == 'messages':
        body['messages'].extend([{'role': 'assistant', 'content': reply['content']}, {'role': 'user', 'content': [{'type': 'tool_result', 'tool_use_id': c['id'], 'content': '{"value":"probe"}'} for c in reply['content'] if c.get('type') == 'tool_use']}])
    elif protocol == 'responses':
        body['input'] = [{'role': 'user', 'content': initial['input']}] + reply['output'] + [{'type': 'function_call_output', 'call_id': c['call_id'], 'output': '{"value":"probe"}'} for c in reply['output'] if c.get('type') == 'function_call']
    else:
        content = reply['candidates'][0]['content']; body['contents'].extend([content, {'role': 'user', 'parts': [{'functionResponse': {'id': c['functionCall']['id'], 'name': 'audit_echo', 'response': {'value': 'probe'}}} for c in content['parts'] if 'functionCall' in c]}])
    return body


def exchange(base, token, protocol, model, body, raw, private_json, streaming=False, tools=False):
    path = {'chat': '/v1/chat/completions', 'messages': '/v1/messages', 'responses': '/v1/responses',
            'gemini': '/v1beta/models/' + urllib.parse.quote(model, safe='') + (':streamGenerateContent?alt=sse' if streaming else ':generateContent')}[protocol]
    if protocol != 'gemini': body['stream'] = streaming
    headers = {'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json'}
    request = urllib.request.Request(base + path, data=json.dumps(body).encode(), headers=headers)
    started = time.monotonic()
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    try:
        with opener.open(request, timeout=45) as response:
            if not streaming:
                data = response.read(2 * 1024 * 1024 + 1)
                if len(data) > 2 * 1024 * 1024: return {'status': 'invalid_response'}, None
                try: parsed = json.loads(data)
                except ValueError: return {'status': 'invalid_response', 'http_status': response.status}, None
                private_json(raw, parsed)
                return {'http_status': response.status, 'elapsed_ms': int((time.monotonic() - started) * 1000)}, parsed
            events, kind, lines, size = [], 'message', [], 0
            for line in response:
                size += len(line)
                if size > 2 * 1024 * 1024: break
                line = line.decode('utf-8').rstrip('\r\n')
                if line.startswith('event:'): kind = line[6:].strip()
                elif line.startswith('data:'): lines.append(line[5:].lstrip())
                elif not line and lines:
                    payload = '\n'.join(lines); lines = []
                    events.append(('done', None) if payload == '[DONE]' else (kind, json.loads(payload)))
                    kind = 'message'
                    if validate_events(protocol, events, tools=tools)['status'] == 'passed': break
            private_json(raw, events)
            result = validate_events(protocol, events, tools=tools)
            result.update(http_status=response.status, elapsed_ms=int((time.monotonic() - started) * 1000))
            return result, None
    except urllib.error.HTTPError as error:
        data = error.read(2 * 1024 * 1024)
        # Bodies stay private; public classification only depends on the status.
        private_json(raw, {'http_status': error.code, 'body': data.decode('utf-8', errors='replace')})
        return {'status': {401: 'unauthorized', 403: 'unauthorized', 429: 'rate_limited', 422: 'capability_rejected'}.get(error.code, 'unexpected_http_status'), 'http_status': error.code}, None
    except (urllib.error.URLError, TimeoutError, OSError, ValueError):
        return {'status': 'transport_or_invalid_response'}, None


@contextmanager
def relay_host(binary, root, account, catalog, private_json):
    root.mkdir(mode=0o700)
    config = root / '.shiju-harness-dev'; (config / 'database').mkdir(parents=True)
    import sqlite3
    model = {'id': account['model_api_id'], 'status': 'active', 'reasoning': catalog.get('reasoning', False), 'tool_call': catalog.get('tool_call', False)}
    for key in ('reasoning_options', 'interleaved', 'modalities', 'limit', 'protocols'):
        if key in catalog: model[key] = catalog[key]
    # This bounded host exercises the reviewed Chat account operation. The raw
    # catalog SDK name is not the runtime wire adapter; explicit native fixture
    # candidates below are still selected by the shared production router.
    payload = {'providers': {account['provider_id']: {'api': account['endpoint'], 'adapter': 'openai-compatible', 'env': [], 'models': {account['model_key']: model}}}}
    database = config / 'database/vibe-window.sqlite3'
    with sqlite3.connect(str(database)) as connection:
        connection.execute('CREATE TABLE system_configs(platform_id TEXT NOT NULL,company_id TEXT NOT NULL,payload_json TEXT NOT NULL,updated_at_ms INTEGER NOT NULL,PRIMARY KEY(platform_id,company_id))')
        connection.execute('INSERT INTO system_configs VALUES(?,?,?,?)', ('10000', '100000000', json.dumps(payload), int(time.time() * 1000)))
    os.chmod(database, 0o600)
    private_json(root / 'seed.json', {k: account[k] for k in ('provider_id', 'model_key', 'credential')})
    environment = os.environ.copy(); environment.update(VIBEWINDOW_TEST_ROOT=str(root), VIBEWINDOW_CONFIG_DIR=str(config))
    log_path = root / 'relay.log'; descriptor = os.open(str(log_path), os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, 'w') as log:
        process = subprocess.Popen([str(binary)], cwd=str(root), env=environment, stdout=log, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 45
            while not (root / 'ready.json').exists():
                if process.poll() is not None: raise RuntimeError('production relay host failed (exit %s)' % process.returncode)
                if time.monotonic() >= deadline: raise RuntimeError('production relay host startup timed out')
                time.sleep(0.1)
            ready = json.loads((root / 'ready.json').read_text())
            yield ready['base_url'], ready['token']
        finally:
            process.terminate()
            try: process.wait(timeout=15)
            except subprocess.TimeoutExpired: process.kill(); process.wait()


def verify_relay(binary, root, account, catalog, private_json):
    attempts = []
    with relay_host(binary, root, account, catalog, private_json) as (base, token):
        for protocol in ('chat', 'messages', 'responses', 'gemini'):
            if protocol == 'gemini':
                default, reply = exchange(base, token, protocol, account['model_key'], request_body(protocol, account['model_key']), root / 'gemini-default.json', private_json)
                if reply is not None: default.update(validate_json(protocol, reply))
                default.update(kind='public_default_text', source_protocol=protocol, thinking_mode='request_default'); attempts.append(default)
            initial = text_request(protocol, account['model_key'])
            result, reply = exchange(base, token, protocol, account['model_key'], initial, root / (protocol + '-text.json'), private_json)
            if reply is not None: result.update(validate_json(protocol, reply))
            result.update(kind='public_text', source_protocol=protocol, thinking_mode='explicit_disabled' if protocol == 'gemini' else 'protocol_default'); attempts.append(result)
            if result.get('status') == 'rate_limited': break
            if result.get('status') != 'passed': continue
            stream = text_request(protocol, account['model_key'])
            result, _ = exchange(base, token, protocol, account['model_key'], stream, root / (protocol + '-stream.json'), private_json, streaming=True)
            result.update(kind='public_streaming', source_protocol=protocol); attempts.append(result)
            if result.get('status') == 'rate_limited': break
            if not catalog.get('tool_call'): continue
            body = text_request(protocol, account['model_key'], tools=True)
            first, reply = exchange(base, token, protocol, account['model_key'], body, root / (protocol + '-tool-call.json'), private_json)
            if reply is not None: first.update(validate_json(protocol, reply, tools=True))
            first.update(kind='public_tool_call', source_protocol=protocol); attempts.append(first)
            if first.get('status') == 'rate_limited': break
            if first.get('status') != 'passed': continue
            body = replay_body(protocol, body, reply)
            final, reply = exchange(base, token, protocol, account['model_key'], body, root / (protocol + '-tool-result.json'), private_json)
            if reply is not None: final.update(validate_json(protocol, reply))
            final.update(kind='public_tool_result', source_protocol=protocol); attempts.append(final)
            if final.get('status') == 'rate_limited': break
            if protocol == 'messages' and catalog.get('reasoning'):
                body = request_body(protocol, account['model_key'], tools=True)
                body['thinking'] = {'type': 'enabled'}
                first, reply = exchange(base, token, protocol, account['model_key'], body, root / 'messages-thinking-call.json', private_json)
                if reply is not None: first.update(validate_json(protocol, reply, tools=True, require_thinking=True))
                first.update(kind='public_thinking_tool_call', source_protocol=protocol, thinking_mode='enabled'); attempts.append(first)
                if first.get('status') == 'passed':
                    body = replay_body(protocol, body, reply)
                    final, reply = exchange(base, token, protocol, account['model_key'], body, root / 'messages-thinking-result.json', private_json)
                    if reply is not None: final.update(validate_json(protocol, reply))
                    final.update(kind='public_thinking_tool_result', source_protocol=protocol, thinking_mode='enabled'); attempts.append(final)
        for result in attempts:
            result['listener'] = base
            result['requested_model_key'] = account['model_key']
            result.setdefault('thinking_mode', 'explicit_disabled' if result['source_protocol'] == 'gemini' else 'protocol_default')
    return attempts
