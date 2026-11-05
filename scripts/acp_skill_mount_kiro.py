"""Wire-only model fixture for the real Kiro V3 progressive context tool."""
import json
from pathlib import Path
import re
import secrets
import shlex


def response(fixture, body):
    messages = []
    for item in body.get('input', []):
        if item.get('type') == 'function_call_output':
            messages.append({'role': 'tool', 'tool_call_id': item.get('call_id'), 'content': item.get('output', '')})
        elif item.get('role'):
            messages.append(item)
    if 'messages' in body:
        messages = body['messages']
    tools = [tool.get('function', tool) for tool in body.get('tools', [])]
    with fixture.lock:
        fixture.records.append(True)
        if fixture.phase_first:
            text = json.dumps(messages)
            if len(fixture.records) == 1:
                fixture.first_prompt_clean = fixture.secret not in text and fixture.body_marker not in text
            description = '\n'.join(t.get('description', '') for t in tools if t.get('name') == 'disclose_context')
            names = set(re.findall(r'name: "([^"\n]+)"', description.replace('\\"', '"')))
            expected = set() if fixture.mode == 'absent' else {fixture.name}
            if fixture.mode == 'added':
                expected.add(fixture.name + '-added')
            fixture.native_catalog = {'observed_count': len(names), 'expected_count': len(expected), 'selected_present': fixture.name in names, 'observed_names': sorted(names), 'tool_names': [t.get('name') for t in tools], 'description_bytes': len(description)}
            if names != expected:
                raise ValueError('Kiro native catalog differs from selected skills')
            fixture.discovery += int(bool(names))
            fixture.discovery_observations.append({'phase': fixture.mode, 'mounted_view': bool(names)})
            fixture.phase_first = False
            fixture.discovered_dir = None
        if fixture.mode == 'absent':
            fixture.discovery_observations[-1]['native_activation_unregistered'] = True
            return {'role': 'assistant', 'content': 'SKILL_ABSENT'}, 'stop'
        for message in messages:
            call = message.get('tool_call_id')
            kind = fixture.calls.get(call)
            if not kind:
                continue
            content = str(message.get('content', ''))
            event = fixture.call_events[call]
            matched = ('ADDED_SKILL_BODY' in content if kind == 'added' else fixture.skill.read_text().split('---\n')[-1].strip() in content if kind == 'skill' else fixture.secret in content)
            if not event['result_received']:
                fixture.total_results += 1
            event.update(result_received=True, matched=matched)
            if not matched:
                fixture.native_catalog['unmatched_kind'] = kind
                fixture.native_catalog['result_error_markers'] = [word for word in ['denied', 'permission', 'outside', 'not found', 'no such', 'error', 'approved', 'not allowed', 'invalid', 'symlink'] if word in content.lower()]
                fixture.native_catalog['result_shape'] = type(message.get('content')).__name__
                fixture.native_catalog['result_diagnostic'] = re.sub(r'https?://[^\s"\']+|/[^\s"\']+|[A-Za-z0-9_=-]{24,}', '[redacted]', content)[:400]
                raise ValueError('Kiro native tool result did not match the synthetic fixture')
            fixture.results.add(kind)
            fixture.completed_kinds.add(kind)
            if kind == 'added':
                fixture.added_confirmed = True
                fixture.discovery_observations[-1]['added_native_verified'] = True
            if kind == 'skill':
                paths = re.findall(r'(/[^\s<>"\x27`\\]*' + re.escape('/' + fixture.name) + r')', content)
                fixture.discovered_dir = next((Path(p) for p in paths if Path(p) != fixture.directory and (Path(p) / 'SKILL.md').is_file() and (Path(p) / 'SKILL.md').read_bytes() == fixture.skill.read_bytes()), None)
                if fixture.discovered_dir is None:
                    raise ValueError('Kiro activation did not identify the actual mounted view')
        if fixture.mode == 'added' and not fixture.added_confirmed:
            kind, name, args = 'added', 'disclose_context', {'name': fixture.name + '-added'}
        elif 'skill' not in fixture.results:
            kind, name, args = 'skill', 'disclose_context', {'name': fixture.name}
        elif 'read' not in fixture.results:
            kind, name, args = 'read', 'read_file', {'path': str(fixture.discovered_dir / 'references/value.txt')}
        elif 'execute' not in fixture.results:
            kind, name, args = 'execute', 'execute_bash', {'command': '/bin/sh ' + shlex.quote(str(fixture.discovered_dir / 'scripts/read.sh'))}
        else:
            return {'role': 'assistant', 'content': 'SKILL_MOUNT_REPLY ' + fixture.secret}, 'stop'
        tool = next((t for t in tools if t.get('name') == name), None)
        if tool is None:
            raise ValueError('Kiro required native tool missing')
        props = tool.get('parameters', {}).get('properties', {})
        if kind == 'read' and 'path' not in props:
            key = next((k for k in ('file_path', 'absolute_path', 'filePath') if k in props), None)
            if key:
                args[key] = args.pop('path')
        if any(k not in args for k in tool.get('parameters', {}).get('required', [])):
            raise ValueError('Kiro native tool schema requires unreviewed arguments')
        call = 'mount_' + secrets.token_hex(12)
        fixture.calls[call] = kind
        fixture.call_events[call] = {'phase': fixture.mode, 'kind': kind, 'result_received': False, 'matched': False}
        fixture.total_calls += 1
        if name == 'disclose_context':
            fixture.native_skill_invocations += 1
        return {'role': 'assistant', 'content': None, 'tool_calls': [{'id': call, 'type': 'function', 'function': {'name': name, 'arguments': json.dumps(args)}}]}, 'tool_calls'
