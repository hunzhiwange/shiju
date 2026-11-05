#!/usr/bin/env python3
"""Production stdio ACP mount acceptance; only the external model HTTP is simulated."""
import json
from collections import Counter
from pathlib import Path
import queue
import shlex
import sqlite3
import sys
import threading
import time
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from verify_acp_integration import Process, ProviderHandler, prepare, require, text_updates


def rpc(request, attachment, audit, transport):
    method = request.get('method')
    if method == 'initialize':
        result = {'protocolVersion': request['params']['protocolVersion'],
                  'capabilities': {'tools': {}}, 'serverInfo': {'name': 'mount-probe', 'version': '1'}}
    elif method == 'tools/list':
        result = {'tools': [{'name': 'read_probe', 'description': 'Read random disk attachment',
                             'inputSchema': {'type': 'object', 'properties': {}}}]}
    elif method == 'tools/call':
        require(request['params']['name'] == 'read_probe', 'unexpected MCP tool')
        value = Path(attachment).read_text()
        with Path(audit).open('a') as out:
            out.write(transport + '\n')
        result = {'content': [{'type': 'text', 'text': value}], 'isError': False}
    else:
        result = {}
    if 'id' in request:
        return {'jsonrpc': '2.0', 'id': request['id'], 'result': result}


class McpHandler(BaseHTTPRequestHandler):
    protocol_version = 'HTTP/1.1'

    def log_message(self, *_args):
        pass

    def reply(self, code, value=None):
        data = json.dumps(value).encode() if value is not None else b''
        self.send_response(code)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path != '/sse':
            self.reply(405)
            return
        key = uuid.uuid4().hex
        messages = queue.Queue()
        self.server.channels[key] = messages
        self.send_response(200)
        self.send_header('Content-Type', 'text/event-stream')
        self.send_header('Cache-Control', 'no-cache')
        self.end_headers()
        try:
            self.wfile.write(f'event: endpoint\ndata: /messages/{key}\n\n'.encode())
            self.wfile.flush()
            while not self.server.stopping.is_set():
                try:
                    item = messages.get(timeout=.3)
                except queue.Empty:
                    continue
                self.wfile.write(('event: message\ndata: ' + json.dumps(item) + '\n\n').encode())
                self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            pass
        finally:
            self.server.channels.pop(key, None)
            self.close_connection = True

    def do_POST(self):
        request = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        transport = 'sse' if self.path.startswith('/messages/') else 'http'
        response = rpc(request, self.server.attachment, self.server.audit, transport)
        if transport == 'sse':
            messages = self.server.channels.get(self.path.rsplit('/', 1)[-1])
            if messages is None:
                self.reply(404)
                return
            if response:
                messages.put(response)
            self.reply(202)
        else:
            self.reply(200 if response else 202, response)

    def do_DELETE(self):
        self.reply(204)


class ModelHandler(ProviderHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        messages = body['messages']
        start = max(i for i, item in enumerate(messages) if item.get('role') == 'user')
        user = messages[start]['content']
        if isinstance(user, list):
            user = '\n'.join(item.get('text', '') for item in user)
        marker = 'ACCEPTANCE_REQUEST '
        try:
            spec = json.JSONDecoder().raw_decode(user.split(marker, 1)[1])[0]
            results = [item for item in messages[start + 1:] if item.get('role') == 'tool']
            tools = {item['function']['name'] for item in body.get('tools', [])}
            for version in ('V1', 'V2'):
                if 'BODY_' + version in json.dumps(messages):
                    self.server.skill_bodies.add(version)
            if spec.get('source') and not results and not self.server.checked_initial_body:
                require('BODY_V1' not in json.dumps(messages), 'skill body injected before invocation')
                self.server.checked_initial_body = True
            plan = [('skill', {'skill': 'mount_probe'})]
            if spec.get('source'):
                source = Path(spec['source'])
                plan += [('file_read', {'path': str(source / 'references/probe.txt')}),
                         ('bash', {'command': 'sh ' + shlex.quote(str(source / 'scripts/probe.sh'))})]
            if spec.get('mcp'):
                plan += [('tool_search', {'query': 'read_probe', 'limit': 10})]
            plan += [(name, {}) for name in spec.get('mcp', [])]
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            if spec.get('slow'):
                self.server.started.set()
                for _ in range(200):
                    self.chunk(body['model'], {'content': 'waiting '}, None)
                    time.sleep(.1)
                self.chunk(body['model'], {}, 'stop')
            elif len(results) < len(plan):
                name, arguments = plan[len(results)]
                # No body is returned by the model until the actual tool has run.
                if name not in tools:
                    self.chunk(body['model'], {'content': 'TOOL_UNAVAILABLE:' + name}, None)
                    self.chunk(body['model'], {}, 'stop')
                else:
                    call = {'index': 0, 'id': 'mount_' + uuid.uuid4().hex,
                            'type': 'function', 'function': {'name': name, 'arguments': json.dumps(arguments)}}
                    self.chunk(body['model'], {'tool_calls': [call]}, None)
                    self.chunk(body['model'], {}, 'tool_calls')
            else:
                self.server.turns[spec['probeId']] = results
                content = '\n'.join(str(item.get('content', '')) for item in results)
                self.chunk(body['model'], {'content': content}, None)
                self.chunk(body['model'], {}, 'stop')
            self.wfile.write(b'data: [DONE]\n\n')
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            pass
        except Exception as error:
            self.server.errors.append(type(error).__name__ + ': ' + str(error))
            raise


class Agent(Process):
    def receive(self, request_id, error=False, on_frame=None):
        frames = []
        deadline = time.monotonic() + 90
        while True:
            frame = self.frames.get(timeout=max(.01, deadline - time.monotonic()))
            require('eof' not in frame and 'invalid_stdout' not in frame, 'agent exited or invalid stdout')
            if frame.get('method') == 'session/request_permission':
                # This client authorizes only the isolated test workspace's tool requests.
                option = next(item for item in frame['params']['options'] if item['kind'] == 'allow_once')
                self.child.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': frame['id'],
                    'result': {'outcome': {'outcome': 'selected', 'optionId': option['optionId']}}}) + '\n')
                self.child.stdin.flush()
            if frame.get('id') == request_id and 'method' not in frame:
                require(('error' in frame) == error, str(frame))
                return frame.get('error') if error else frame['result'], frames
            frames.append(frame)
            if on_frame:
                on_frame(frame)

    def request(self, method, params, error=False):
        return self.receive(self.send(method, params), error=error)


def selection(source):
    return {'shiju.skillMounts': {'version': 1, 'skills': [
        {'name': 'mount_probe', 'path': str(source)}] if source else []}}


def mounts(result):
    return result['_meta']['shiju.mounts']


def run(shiju, output):
    """Return explicit stage results, retaining failed stages in a redacted report."""
    output.mkdir(parents=True, exist_ok=True)
    report = {'production_starts': 0, 'checks': {}, 'passed': False, 'processes': []}
    agents = []
    servers = []
    def check(name, action):
        try:
            value = action()
            report['checks'][name] = {'passed': True}
            return value
        except Exception as error:
            report['checks'][name] = {'passed': False, 'error': str(error)}
            raise
    try:
        model = ThreadingHTTPServer(('127.0.0.1', 0), ModelHandler)
        model.daemon_threads = True
        model.started = threading.Event()
        model.errors = []
        model.skill_bodies = set()
        model.checked_initial_body = False
        model.turns = {}
        servers.append(model)
        threading.Thread(target=model.serve_forever, daemon=True).start()
        env, cwd = prepare(output, model.server_port)
        database = Path(env['VIBEWINDOW_CONFIG_DIR']) / 'database/vibe-window.sqlite3'
        with sqlite3.connect(database) as connection:
            connection.execute('CREATE TABLE gateway_settings (platform_id TEXT NOT NULL, company_id TEXT NOT NULL, '
                               'namespace TEXT NOT NULL, config_key TEXT NOT NULL, payload_json TEXT NOT NULL, '
                               'updated_at_ms INTEGER NOT NULL, PRIMARY KEY(platform_id,company_id,namespace,config_key))')
            connection.execute('INSERT INTO gateway_settings VALUES (?,?,?,?,?,?)',
                ('10000', '100000000', 'desktop.preferences', 'default',
                 json.dumps({'permission': {'skill': {'mount_probe': 'allow'}}}), 1))
        # Skill files are outside cwd: native mounting must grant the narrow read scope.
        sources = []
        tokens = []
        for version in ('V1', 'V2'):
            source = output / version
            (source / 'references').mkdir(parents=True)
            (source / 'scripts').mkdir()
            token = version + '_' + uuid.uuid4().hex
            tokens.append(token)
            (source / 'references/probe.txt').write_text(token)
            (source / 'SKILL.md').write_text('---\nname: mount_probe\ndescription: Read mount probe references and run script\n---\n'
                + f'BODY_{version}: Read references/probe.txt and run scripts/probe.sh.\n')
            (source / 'scripts/probe.sh').write_text('printf executed >> ' + shlex.quote(str(cwd / ('executed_' + version)))
                + '\ncat ' + shlex.quote(str(source / 'references/probe.txt')) + '\n')
            sources.append(source)
        audit = output / 'mcp-audit.txt'
        mcp = ThreadingHTTPServer(('127.0.0.1', 0), McpHandler)
        mcp.daemon_threads = True
        mcp.attachment = sources[0] / 'references/probe.txt'
        mcp.audit = audit
        mcp.channels = {}
        mcp.stopping = threading.Event()
        servers.append(mcp)
        threading.Thread(target=mcp.serve_forever, daemon=True).start()
        credential = 'fixture-only-' + uuid.uuid4().hex
        configs = [
            {'name': 'stdio_probe', 'command': sys.executable,
             'args': [str(Path(__file__).resolve()), '--mcp-stdio', str(mcp.attachment), str(audit)], 'env': [{'name': 'ACP_MOUNT_SECRET', 'value': credential}]},
            {'name': 'http_probe', 'type': 'http', 'url': f'http://127.0.0.1:{mcp.server_port}/mcp', 'headers': [{'name': 'X-Mount-Test', 'value': credential}]},
            {'name': 'sse_probe', 'type': 'sse', 'url': f'http://127.0.0.1:{mcp.server_port}/sse', 'headers': [{'name': 'X-Mount-Test', 'value': credential}]}]
        def connect():
            agent = Agent([str(shiju), 'acp'], cwd, env, output / f'agent-{len(agents)}.log')
            agents.append(agent)
            report['production_starts'] += 1
            report['processes'].append({'pid': agent.child.pid})
            result, _ = agent.request('initialize', {'protocolVersion': 1, 'clientCapabilities': {}})
            require(result['_meta']['shiju.mounts']['setMounts'], 'mount capability')
            return agent
        client = check('initialize', connect)
        def create(source):
            result, _ = client.request('session/new', {'cwd': str(cwd), 'mcpServers': configs if source else [],
                                                      '_meta': selection(source)})
            client.request('session/set_model', {'sessionId': result['sessionId'], 'modelId': 'fixture/alpha'})
            return result
        created = check('new_skill_and_three_mcp', lambda: create(sources[0]))
        sid = created['sessionId']
        revision = mounts(created)['revision']
        def probe(source, expected, mcp_names=(), session=sid):
            probe_id = uuid.uuid4().hex
            prompt = 'Inspect the mounted probe resources. ACCEPTANCE_REQUEST ' + json.dumps(
                {'source': str(source) if source else None, 'mcp': list(mcp_names), 'probeId': probe_id})
            result, events = client.request('session/prompt', {'sessionId': session, 'prompt': [{'type': 'text', 'text': prompt}]})
            text = ''.join(text_updates(events))
            require(result['stopReason'] == 'end_turn', 'probe stop reason')
            if source and text.startswith('TOOL_UNAVAILABLE:'):
                raise AssertionError(text.splitlines()[0])
            for value in expected:
                if value.startswith('BODY_'):
                    require(value[5:] in model.skill_bodies, 'skill body did not reach model after native invocation')
                    continue
                require(value in text, 'random tool evidence missing')
            if source:
                outputs = model.turns.get(probe_id, [])
                require(len(outputs) >= 3, 'native tool round incomplete')
                require('loaded' in str(outputs[0].get('content', '')), 'native Skill invocation failed')
                for index, name in ((1, 'file read'), (2, 'script execution')):
                    require(expected[0] in str(outputs[index].get('content', '')), name + ' did not return random attachment')
                for item in outputs[4:] if mcp_names else []:
                    require(tokens[0] in str(item.get('content', '')), 'MCP tool did not return disk attachment')
            return text
        def absent():
            empty = create(None)
            text = probe(None, (), session=empty['sessionId'])
            require('BODY_V1' not in text and tokens[0] not in text, 'unmounted skill leaked')
            require('not found' in text.lower() or 'unavailable' in text.lower() or '未找到技能' in text, 'negative skill evidence absent')
            try:
                probe(None, [tokens[0]], ['mcp__stdio_probe__read_probe'], session=empty['sessionId'])
            except AssertionError as error:
                require(str(error) == 'random tool evidence missing', 'unexpected negative failure')
            else:
                raise AssertionError('unmounted session unexpectedly produced random attachment/MCP evidence')
        check('missing_mount_negative', absent)
        def initial():
            probe(sources[0], [tokens[0], 'BODY_V1'], [f'mcp__{kind}_probe__read_probe' for kind in ('stdio', 'http', 'sse')])
            require((cwd / 'executed_V1').read_text() == 'executed', 'script did not execute')
            require(set(audit.read_text().splitlines()) == {'stdio', 'http', 'sse'}, 'three actual MCP calls')
        check('native_skill_reference_script_three_transports', initial)
        def update(source, expected_revision, servers=configs, error=False):
            return client.request('_shiju/session/set_mounts', {'sessionId': sid, 'expectedRevision': expected_revision,
                'skills': selection(source)['shiju.skillMounts']['skills'], 'mcpServers': servers}, error=error)[0]
        changed = check('update_v2', lambda: update(sources[1], revision))
        revision = changed['revision']
        check('v2_native_execution', lambda: probe(sources[1], [tokens[1], 'BODY_V2']))
        check('failed_update', lambda: update(output / 'missing', revision, error=True))
        check('failed_mcp_update', lambda: update(sources[0], revision, servers=configs + [
            {'name': 'broken_probe', 'command': str(output / 'missing-mcp'), 'args': [], 'env': []}], error=True))
        check('rollback_keeps_v2', lambda: probe(sources[1], [tokens[1], 'BODY_V2'], ['mcp__stdio_probe__read_probe']))
        check('revision_conflict', lambda: update(sources[0], revision - 1, error=True))
        def removed():
            changed = update(None, revision, servers=[])
            text = probe(None, ())
            require('BODY_V2' not in text and tokens[1] not in text, 'removed skill remains usable')
            require(changed['skills'] == [] and changed['mcpServers'] == [], 'remove did not clear names')
            text = probe(None, (), ['mcp__stdio_probe__read_probe'])
            require(text in ('TOOL_UNAVAILABLE:tool_search', 'TOOL_UNAVAILABLE:mcp__stdio_probe__read_probe'), 'removed MCP still advertised')
            return changed['revision']
        revision = check('remove_all', removed)
        revision = check('remount_v2', lambda: update(sources[1], revision))['revision']
        def busy():
            model.started.clear()
            pending = client.send('session/prompt', {'sessionId': sid, 'prompt': [{'type': 'text',
                'text': 'ACCEPTANCE_REQUEST ' + json.dumps({'slow': True})}]})
            require(model.started.wait(20), 'slow model request not reached')
            error, _ = client.request('_shiju/session/set_mounts', {'sessionId': sid, 'expectedRevision': revision,
                'skills': [], 'mcpServers': []}, error=True)
            require('busy' in json.dumps(error).lower(), 'busy refusal absent')
            client.send('session/cancel', {'sessionId': sid}, notification=True)
            result, _ = client.receive(pending)
            require(result['stopReason'] == 'cancelled', 'cancel stop reason')
        check('busy_reject_and_cancel', busy)
        revision = check('update_after_cancel', lambda: update(sources[1], revision))['revision']
        client.close()
        client = check('restart_real_process', connect)
        def no_mcp_restore():
            loaded, _ = client.request('session/load', {'sessionId': sid, 'cwd': str(cwd), 'mcpServers': []})
            require(mounts(loaded)['skills'] == ['mount_probe'], 'saved skills absent')
            require(mounts(loaded)['mcpServers'] == [], 'MCP restored without retransmission')
        check('load_does_not_restore_mcp_credentials', no_mcp_restore)
        def restore():
            loaded, _ = client.request('session/load', {'sessionId': sid, 'cwd': str(cwd), 'mcpServers': configs})
            require(mounts(loaded)['skills'] == ['mount_probe'], 'saved skills absent')
            require(mounts(loaded)['revision'] == revision, 'revision not restored')
            probe(sources[1], [tokens[1], 'BODY_V2'], ['mcp__stdio_probe__read_probe'])
        check('load_without_skills_retransmit_mcp', restore)
        def clear():
            loaded, _ = client.request('session/load', {'sessionId': sid, 'cwd': str(cwd), 'mcpServers': [], '_meta': selection(None)})
            require(mounts(loaded)['skills'] == [] and mounts(loaded)['mcpServers'] == [], 'explicit clear failed')
            text = probe(None, ())
            require('BODY_V2' not in text, 'cleared skill body leaked')
        check('explicit_empty_clears', clear)
        def credentials_not_persisted():
            for path in (output / 'home').rglob('*'):
                if path.is_file():
                    require(credential.encode() not in path.read_bytes(), 'MCP credential persisted to disk')
        check('credentials_not_persisted', credentials_not_persisted)
        require(not model.errors, 'model fixture errors')
        report['mcp_calls'] = dict(Counter(audit.read_text().splitlines()))
        report['passed'] = True
        return report
    finally:
        for agent in agents:
            agent.close()
        for process, agent in zip(report['processes'], agents):
            process['exit'] = agent.child.returncode
        report['executed_checks'] = len(report['checks'])
        report['passed_checks'] = sum(item['passed'] for item in report['checks'].values())
        for server in servers:
            if hasattr(server, 'stopping'):
                server.stopping.set()
            server.shutdown()
            server.server_close()
        (output / 'mount-results.json').write_text(json.dumps(report, indent=2))


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == '--mcp-stdio':
        for line in sys.stdin:
            response = rpc(json.loads(line), sys.argv[2], sys.argv[3], 'stdio')
            if response:
                print(json.dumps(response), flush=True)
    else:
        import argparse
        parser = argparse.ArgumentParser(description=__doc__)
        parser.add_argument('--shiju', type=Path, required=True)
        parser.add_argument('--output', type=Path, required=True)
        args = parser.parse_args()
        print(json.dumps(run(args.shiju.resolve(strict=True), args.output.resolve())))
