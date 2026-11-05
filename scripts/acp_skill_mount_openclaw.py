"""External model replies only; OpenClaw discovers and reads every skill itself."""
import json
from pathlib import Path
import re
import secrets
import shlex


def response(fixture, body):
    tools = [t.get('function', t) for t in body.get('tools', [])]
    if not tools:
        return {'role': 'assistant', 'content': 'Ready'}, 'stop'
    messages = body.get('messages', [])
    with fixture.lock:
        fixture.records.append(True)
        if fixture.phase_first:
            metadata = '\n'.join(str(m.get('content', '')) for m in messages if m.get('role') == 'system')
            if len(fixture.records) == 1:
                fixture.first_prompt_clean = fixture.secret not in json.dumps(messages) and fixture.body_marker not in json.dumps(messages)
            catalog = '\n'.join(re.findall(r'<available_skills>(.*?)</available_skills>', metadata, re.S))
            names = set(re.findall(r'<name>\s*([^<\s]+)\s*</name>', catalog))
            expected = set() if fixture.mode == 'absent' else {fixture.name}
            if fixture.mode == 'added':
                expected.add(fixture.name + '-added')
            fixture.native_catalog = {'phase': fixture.mode, 'observed_names': sorted(names), 'expected_count': len(expected), 'tool_names': [t.get('name') for t in tools]}
            if names != expected:
                raise ValueError('OpenClaw native catalog differs from selected skills')
            paths = re.findall(r'<location>\s*([^<]+)\s*</location>', catalog)
            fixture.discovered_dir = next((Path(p).parent for p in paths if p.endswith('/' + fixture.name + '/SKILL.md') and Path(p) != fixture.skill and Path(p).is_file() and Path(p).read_bytes() == fixture.skill.read_bytes()), None)
            fixture.discovery += int(bool(names))
            fixture.discovery_observations.append({'phase': fixture.mode, 'mounted_view': bool(names)})
            fixture.phase_first = False
        if fixture.mode == 'absent':
            fixture.discovery_observations[-1]['native_activation_unregistered'] = True
            return {'role': 'assistant', 'content': 'SKILL_ABSENT'}, 'stop'
        if fixture.discovered_dir is None:
            raise ValueError('OpenClaw catalog has no mounted directory')
        for message in messages:
            call = message.get('tool_call_id')
            kind = fixture.calls.get(call)
            if kind not in ('skill', 'added', 'read', 'execute'):
                continue
            content = str(message.get('content', ''))
            event = fixture.call_events[call]
            matched = ('ADDED_SKILL_BODY' in content if kind == 'added' else fixture.skill.read_text().split('---\n')[-1].strip() in content if kind == 'skill' else fixture.secret in content)
            if not event['result_received']:
                fixture.total_results += 1
            event.update(result_received=True, matched=matched)
            if not matched:
                fixture.native_catalog['failed_tool_kind'] = kind
                fixture.native_catalog['error_markers'] = [s for s in ('permission', 'denied', 'not found', 'outside', 'invalid', 'symlink', 'error') if s in content.lower()]
                raise ValueError('OpenClaw tool result did not match synthetic fixture')
            fixture.results.add(kind)
            fixture.completed_kinds.add(kind)
            if kind == 'added':
                fixture.added_confirmed = True
                fixture.discovery_observations[-1]['added_native_verified'] = True
        if fixture.mode == 'added' and not fixture.added_confirmed:
            kind = 'added'
        else:
            kind = next((k for k in ('skill', 'read', 'execute') if k not in fixture.results), None)
        if kind is None:
            return {'role': 'assistant', 'content': fixture.answer(messages)}, 'stop'
        for tool in tools:
            props = tool.get('parameters', {}).get('properties', {})
            if kind == 'execute':
                key = next((k for k in ('command', 'cmd') if k in props), None)
                if not key or not any(k in tool.get('name', '').lower() for k in ('exec', 'shell', 'bash', 'terminal')):
                    continue
                args = {key: '/bin/sh ' + shlex.quote(str(fixture.discovered_dir / 'scripts/read.sh'))}
            else:
                key = next((k for k in ('file_path', 'path', 'absolute_path', 'filePath') if k in props), None)
                if not key or 'read' not in tool.get('name', '').lower():
                    continue
                path = fixture.discovered_dir / ('references/value.txt' if kind == 'read' else 'SKILL.md')
                if kind == 'added':
                    path = fixture.discovered_dir.parent / (fixture.name + '-added') / 'SKILL.md'
                args = {key: str(path)}
            if any(k not in args for k in tool.get('parameters', {}).get('required', [])):
                continue
            # The native OpenAI converter strips punctuation from call IDs.
            call = 'mount' + secrets.token_hex(12)
            fixture.calls[call] = kind
            fixture.call_events[call] = {'phase': fixture.mode, 'kind': kind, 'result_received': False, 'matched': False}
            fixture.total_calls += 1
            fixture.native_skill_invocations += int(kind in ('skill', 'added'))
            return {'role': 'assistant', 'content': None, 'tool_calls': [{'id': call, 'type': 'function', 'function': {'name': tool['name'], 'arguments': json.dumps(args)}}]}, 'tool_calls'
        raise ValueError('OpenClaw required native tool missing')
