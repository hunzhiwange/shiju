#!/usr/bin/env python3
"""Probe unmodified Copilot ACP catalog boundaries, never full mount acceptance.

Only model HTTP is substituted. No CLI, skill loader, ACP transport or filesystem
is mocked. Output contains synthetic observations, never raw model messages.
"""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import queue
import secrets
import subprocess
import tempfile
import time
import threading

from verify_acp_model_source import locate

ROOT = Path(__file__).resolve().parents[1]


def probe(binary, options, metadata=False, env_skills=False, plugin=False, isolated=False):
    observations = []
    key = secrets.token_hex(24)

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_POST(self):
            if self.headers.get('Authorization') != 'Bearer ' + key:
                self.send_error(401)
                return
            size = int(self.headers.get('Content-Length', '0'))
            if not 0 < size <= 2_000_000:
                self.send_error(400)
                return
            body = json.loads(self.rfile.read(size))
            # Only the synthetic names and marker booleans leave this process.
            catalog = json.dumps([m for m in body.get('messages', [])
                                  if m.get('role') in ('system', 'developer')]) + json.dumps(body.get('tools', []))
            observations.append({'selected_in_catalog': 'selected-probe' in catalog,
                                 'unselected_in_catalog': 'ambient-probe' in catalog,
                                 'body_in_initial_request': 'SYNTHETIC_PRIVATE_BODY' in json.dumps(body),
                                 'tool_count': len(body.get('tools', []))})
            message = {'role': 'assistant', 'content': 'Synthetic catalog probe completed.'}
            if body.get('stream'):
                chunks = [{'id': 'probe', 'object': 'chat.completion.chunk', 'model': 'probe',
                           'choices': [{'index': 0, 'delta': message, 'finish_reason': None}]},
                          {'id': 'probe', 'object': 'chat.completion.chunk', 'model': 'probe',
                           'choices': [{'index': 0, 'delta': {}, 'finish_reason': 'stop'}]}]
                data = (''.join('data: ' + json.dumps(c) + '\n\n' for c in chunks) + 'data: [DONE]\n\n').encode()
                content_type = 'text/event-stream'
            else:
                data = json.dumps({'id': 'probe', 'object': 'chat.completion', 'model': 'probe',
                                   'choices': [{'index': 0, 'message': message, 'finish_reason': 'stop'}],
                                   'usage': {'prompt_tokens': 1, 'completion_tokens': 1, 'total_tokens': 2}}).encode()
                content_type = 'application/json'
            self.send_response(200)
            self.send_header('Content-Type', content_type)
            self.send_header('Content-Length', str(len(data)))
            self.end_headers()
            self.wfile.write(data)

    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        with tempfile.TemporaryDirectory(prefix='copilot-acp-skills-contract-') as directory:
            root = Path(directory)
            root.chmod(0o700)
            config, work, selected = root / 'config', root / 'work', root / 'selected'
            config.mkdir(mode=0o700)
            work.mkdir()
            for parent, name in ((selected, 'selected-probe'), (work / '.github/skills', 'ambient-probe')):
                skill = parent / name
                skill.mkdir(parents=True)
                (skill / 'SKILL.md').write_text(f'---\nname: {name}\ndescription: Synthetic contract probe\n---\nSYNTHETIC_PRIVATE_BODY\n')
            settings = {'model': 'probe/probe', 'autoUpdate': False}
            if not metadata:
                settings['skillDirectories'] = [str(selected)]
            settings.update(options)
            if isolated:
                settings['ignoredSkillsLocations'] = [str(work.resolve()), str((config / 'skills').resolve()),
                    str((Path.home() / '.agents/skills').resolve()), str((Path.home() / '.copilot/skills').resolve())]
                # Query only the real CLI's builtin names; ambient skill names are not persisted.
                native_env = dict(os.environ, COPILOT_HOME=str(config), COPILOT_OFFLINE='true', COPILOT_AUTO_UPDATE='false')
                listed = subprocess.run([binary, '--no-auto-update', 'skill', 'list', '--json'], cwd=work,
                                        env=native_env, capture_output=True, text=True, timeout=20, check=True)
                settings['disabledSkills'] = [x['name'] for x in json.loads(listed.stdout) if x['source'] == 'builtin']
            (config / 'settings.json').write_text(json.dumps(settings))
            (config / 'providers.json').write_text(json.dumps({
                'providers': [{'name': 'probe', 'type': 'openai',
                               'baseUrl': f'http://127.0.0.1:{server.server_port}/v1', 'apiKey': key}],
                'models': [{'id': 'probe', 'provider': 'probe', 'wireModel': 'probe'}]}))
            (config / 'providers.json').chmod(0o600)
            env = {k: os.environ[k] for k in ('HOME', 'PATH', 'TMPDIR', 'USER') if k in os.environ}
            env.update(COPILOT_HOME=str(config), COPILOT_OFFLINE='true', COPILOT_AUTO_UPDATE='false')
            if env_skills:
                env['COPILOT_SKILLS_DIRS'] = str(selected)
            extra_args = []
            if plugin:
                manifest = selected / '.github/plugin'
                manifest.mkdir(parents=True)
                (manifest / 'plugin.json').write_text(json.dumps({'name': 'synthetic-probe', 'version': '1.0.0', 'skills': './'}))
                extra_args = ['--plugin-dir', str(selected)]
            child = subprocess.Popen([binary, '--no-auto-update', '--acp', '--stdio', *extra_args], cwd=work, env=env,
                                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                                     text=True, bufsize=1)
            lines = queue.Queue()
            reader = threading.Thread(target=lambda: [lines.put(line) for line in child.stdout], daemon=True)
            reader.start()
            def request(identifier, method, params, allow_error=False):
                child.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': identifier,
                                              'method': method, 'params': params}) + '\n')
                child.stdin.flush()
                # Bounded total protocol time, including notifications.
                end = time.monotonic() + 30
                while time.monotonic() < end:
                    row = json.loads(lines.get(timeout=max(0.01, end - time.monotonic())))
                    if row.get('id') == identifier:
                        if 'error' in row and not allow_error:
                            raise RuntimeError('native ACP rejected probe request')
                        return row.get('result', row.get('error'))
                    if row.get('method') and 'id' in row:
                        # No tool is requested by the fixture; reject unexpected requests.
                        raise RuntimeError('unexpected native client request')
                raise TimeoutError('native ACP response timeout')
            try:
                initialized = request(1, 'initialize', {'protocolVersion': 1, 'clientCapabilities': {}})
                params = {'cwd': str(work), 'mcpServers': []}
                if metadata:
                    candidates = {'skillDirectories': [str(selected)], 'enableConfigDiscovery': False,
                                  'includedBuiltinSkills': [], 'installedPlugins': []}
                    params.update(candidates)
                    params['_meta'] = candidates
                session = request(2, 'session/new', params)
                directory_option = request(4, 'session/set_config_option', {'sessionId': session['sessionId'],
                    'configId': 'skillDirectories', 'value': str(selected)}, allow_error=True)
                request(3, 'session/prompt', {'sessionId': session['sessionId'],
                    'prompt': [{'type': 'text', 'text': 'Complete this synthetic catalog probe.'}]})
                return {'agent_version': initialized.get('agentInfo', {}).get('version'),
                        'config_options': [x['id'] for x in session.get('configOptions', [])],
                        'directory_config_error_code': directory_option.get('code'),
                        'model_requests': len(observations), 'observations': observations,
                        'tool_requests': 0, 'mount_acceptance_passed': False}
            finally:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
                child.stdin.close()
                reader.join(timeout=5)
                child.stdout.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    entry = next(e for e in json.loads((ROOT / 'docs/shiju/acp-model-source/cli-versions.json').read_text())['entries']
                 if e['key'] == 'copilot')
    binary = locate(entry, os.environ)['command']
    expected = next(e for e in json.loads((ROOT / 'docs/vibewindow/acp-skill-mounts/agents.json').read_text())['entries']
                    if e['key'] == 'copilot')
    digest = hashlib.sha256(Path(binary).read_bytes()).hexdigest()
    if digest not in {e.get('sha256') for e in expected['evidence'] if e.get('kind') == 'binary_static'}:
        raise RuntimeError('native CLI hash differs from reviewed artifacts')
    reports = {'native_sha256': digest,
               'scope': 'Unmodified native ACP catalog probe only; no lifecycle or tool acceptance claim.',
               'additive_settings': probe(binary, {}),
               'sdk_keys_in_settings': probe(binary, {'enableConfigDiscovery': False, 'includedBuiltinSkills': []}),
               'sdk_keys_in_acp_params_and_meta': probe(binary, {}, metadata=True),
               'environment_skill_root': probe(binary, {}, env_skills=True),
               'explicit_plugin_directory': probe(binary, {}, plugin=True),
               'environment_root_with_native_exclusions': probe(binary, {}, env_skills=True, isolated=True)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(reports, indent=2) + '\n')
    print(json.dumps(reports, indent=2))


if __name__ == '__main__':
    main()
