#!/usr/bin/env python3
"""Task-10 runner with a Copilot-specific external model tool fixture.

Only the external model is simulated. The production Rust entry, relay, ACP
transport and installed Copilot binary are exercised by the shared runner.
"""
import json
from pathlib import Path
import sys
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / 'scripts'))
import verify_acp_model_source as runner
from acp_model_source_fixture import ModelFixture


class CopilotFixture(ModelFixture):
    def __init__(self):
        super().__init__()
        fixture = self
        parent = self.server.RequestHandlerClass

        class Handler(parent):
            def do_POST(self):
                try:
                    size = int(self.headers.get('Content-Length', '0'))
                    if not 0 < size <= 2_000_000:
                        raise ValueError('size')
                    body = json.loads(self.rfile.read(size))
                    if (self.path != '/v1/chat/completions'
                            or self.headers.get('Authorization') != 'Bearer ' + fixture.key
                            or body.get('model') not in fixture.models):
                        raise ValueError('routing')
                    messages = body.get('messages', [])
                    results = sum(m.get('role') == 'tool' and fixture.challenge in json.dumps(m.get('content')) for m in messages)
                    child = any('COPILOT_AUXILIARY_CHILD' in json.dumps(m.get('content')) for m in messages if m.get('role') == 'user')
                    with fixture.lock:
                        fixture.records.append({'model': body['model'], 'authenticated': True,
                                                'route': self.path, 'tool_results': results, 'auxiliary': child})
                    if 'CANCEL_PROBE' in json.dumps(next((m for m in reversed(messages) if m.get('role') == 'user'), {})):
                        time.sleep(5)
                    tools = [t.get('function', {}) for t in body.get('tools', [])]
                    tool = next((t for t in tools if t.get('name') == 'view'), None)
                    delegated = any(call.get('function', {}).get('name') == 'task'
                                    for m in messages if m.get('role') == 'assistant'
                                    for call in m.get('tool_calls', []))
                    task = next((t for t in tools if t.get('name') == 'task'), None)
                    if not child and not delegated and task:
                        message = {'role': 'assistant', 'content': None, 'tool_calls': [
                            {'id': 'task-' + str(len(fixture.records)), 'type': 'function', 'function': {
                                'name': 'task', 'arguments': json.dumps({
                                    'name': 'offline-read', 'description': 'Read offline challenge twice',
                                    'agent_type': 'general-purpose', 'mode': 'sync',
                                    'prompt': 'COPILOT_AUXILIARY_CHILD: Read ' + fixture.read_path + ' twice with view, then return the contents.'})}}]}
                        reason = 'tool_calls'
                    elif results < 2:
                        if tool is None or 'path' not in tool.get('parameters', {}).get('properties', {}):
                            raise ValueError('view schema')
                        message = {'role': 'assistant', 'content': None, 'tool_calls': [
                            {'id': 'view-' + str(len(fixture.records)), 'type': 'function', 'function': {
                                'name': 'view', 'arguments': json.dumps({'path': fixture.read_path})}}]}
                        reason = 'tool_calls'
                    else:
                        message = {'role': 'assistant', 'content': 'FIXTURE_MODEL_REPLY ' + fixture.challenge}
                        reason = 'stop'
                    if not body.get('stream'):
                        return self.reply(200, {'id': 'fixture', 'object': 'chat.completion', 'model': body['model'],
                            'choices': [{'index': 0, 'message': message, 'finish_reason': reason}],
                            'usage': {'prompt_tokens': 1, 'completion_tokens': 1, 'total_tokens': 2}})
                    delta = dict(message)
                    if 'tool_calls' in delta:
                        delta['tool_calls'] = [dict(delta['tool_calls'][0], index=0)]
                    chunks = [{'id': 'fixture', 'object': 'chat.completion.chunk', 'model': body['model'],
                               'choices': [{'index': 0, 'delta': delta, 'finish_reason': None}]},
                              {'id': 'fixture', 'object': 'chat.completion.chunk', 'model': body['model'],
                               'choices': [{'index': 0, 'delta': {}, 'finish_reason': reason}],
                               'usage': {'prompt_tokens': 1, 'completion_tokens': 1, 'total_tokens': 2}}]
                    data = (''.join('data: ' + json.dumps(c) + '\n\n' for c in chunks) + 'data: [DONE]\n\n').encode()
                    self.send_response(200)
                    self.send_header('Content-Type', 'text/event-stream')
                    self.send_header('Content-Length', str(len(data)))
                    self.end_headers()
                    self.wfile.write(data)
                except (BrokenPipeError, ConnectionResetError):
                    pass
                except (ValueError, TypeError, KeyError):
                    with fixture.lock:
                        fixture.errors += 1
                    self.reply(400, {'error': {'message': 'Copilot fixture contract rejected request'}})

        self.server.RequestHandlerClass = Handler


def locate_copilot(entry, env):
    # 1.0.105 prints a trailing period; the generic version regex rejects it.
    command = shutil.which('copilot', path=env.get('PATH'))
    if not command:
        raise runner.Unavailable('real Copilot CLI is not installed')
    release = entry['distribution']['release']
    probe = subprocess.run([command, '--version'], env=env, capture_output=True, timeout=20)
    if probe.returncode or probe.stdout.decode().splitlines()[0] != 'GitHub Copilot CLI ' + release + '.':
        raise runner.VersionMismatch('installed Copilot version differs from manifest')
    if entry['underlying'] != {'kind': 'same_distribution', 'release': release}:
        raise runner.Unavailable('unverified Copilot distribution')
    return {'command': str(Path(command).absolute()), 'args': entry['acp_launch'][1:], 'env': {}}


if __name__ == '__main__':
    runner.locate = locate_copilot
    runner.ModelFixture = CopilotFixture
    base_successful = runner.successful
    runner.successful = lambda evidence, flow: base_successful(evidence, flow) and any(
        row.get('auxiliary') is True for row in evidence.get('requests', []))
    result = runner.main()
    output = Path(sys.argv[sys.argv.index('--output') + 1])
    report = json.loads(output.read_text())
    report['auxiliary_verified'] = any(row.get('auxiliary') is True
        for row in report.get('requests', {}).get('requests', []))
    output.write_text(json.dumps(report, indent=2) + '\n')
    raise SystemExit(result)
