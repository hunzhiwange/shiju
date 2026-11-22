"""Expose tools from Grok's actual ACP stdio MCP connections to the bound model."""
import fcntl
import hashlib
import hmac
import http.client
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

USE = 'use_tool'
DISCOVERY = {'search_tool', USE}
MAX_FRAME = 64 * 1024 * 1024
TOKEN = 'SHIJU_GROK_RELAY_KEY'
SERVER_NAME = re.compile(r'[A-Za-z_][A-Za-z0-9_-]*')
TOOL_NAME = re.compile(r'[A-Za-z0-9_-]+')
SOURCE_PATH = re.compile(r'[A-Za-z0-9_./-]+')
SESSIONS = {'session/new', 'session/load', 'session/resume'}


def encode(value):
    return json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode()


def publish(path, value):
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as out:
        staging = Path(out.name)
        try:
            out.write(value if isinstance(value, bytes) else encode(value))
            out.close()
            staging.replace(path)
        finally:
            staging.unlink(missing_ok=True)


def read_catalog(path):
    try:
        return json.loads(path.read_bytes())
    except FileNotFoundError:
        return None


def lines(stream):
    while line := stream.readline(MAX_FRAME + 1):
        if len(line) > MAX_FRAME:
            raise ValueError('ACP or relay frame exceeded the size limit')
        yield line


def send(stream, line):
    stream.write(line)
    stream.flush()


def host_tool_id(server, tool):
    # Match tool_mcp::mcp_tool_id for host allow/deny entries.
    server_id = re.sub(r'[^A-Za-z0-9]+', '_', server).strip('_').lower()
    tool_id = re.sub(r'[^A-Za-z0-9]+', '_', tool).strip('_').lower()
    if not server_id or not tool_id:
        return None
    name = f'mcp__{server_id}__{tool_id}'
    if server != server_id or tool != tool_id or len(name) > 64:
        digest = hashlib.sha256(f'{server}\0{tool}'.encode()).hexdigest()[:16]
        return f'{name[:47]}_{digest}'
    return name


class Catalog:
    """Publish only complete tools/list results from the live native connection."""

    def __init__(self, path, policy=None, server=None):
        self.path = path
        self.server = server
        self.allowed = set((policy or {}).get('allow') or [])
        self.denied = set((policy or {}).get('deny') or [])
        self.requests = {}
        self.initialize_id = None
        self.pages = {}
        self.generation = 0

    def admits(self, name):
        if not isinstance(name, str):
            return False
        keys = {name}
        if self.server:
            identifier = host_tool_id(self.server, name)
            if identifier is None:
                return False
            keys.add(identifier)
        return not keys & self.denied and (not self.allowed or bool(keys & self.allowed))

    def request(self, msg):
        if 'id' not in msg:
            return
        if msg.get('method') == 'initialize':
            self.initialize_id = msg['id']
        elif msg.get('method') == 'tools/list':
            self.requests[msg['id']] = (self.generation, (msg.get('params') or {}).get('cursor'))

    def response(self, msg):
        if msg.get('method') == 'notifications/tools/list_changed':
            self.path.unlink(missing_ok=True)
            self.pages.clear()
            self.generation += 1
        if 'method' in msg or 'id' not in msg:
            return
        result = msg.get('result') or {}
        if self.initialize_id is not None and msg['id'] == self.initialize_id:
            self.initialize_id = None
            if 'capabilities' in result and 'tools' not in result['capabilities']:
                publish(self.path, [])
            return
        if msg['id'] not in self.requests:
            return
        generation, cursor = self.requests.pop(msg['id'])
        if generation != self.generation or 'tools' not in result:
            return
        tools = [tool for tool in result['tools'] if self.admits(tool.get('name'))]
        result['tools'] = tools
        if cursor and cursor not in self.pages:
            return
        tools = self.pages.pop(cursor, []) + tools
        if result.get('nextCursor'):
            self.pages[result['nextCursor']] = tools
        else:
            publish(self.path, tools)


def forward_mcp(path, command):
    policy_file = path.with_suffix('.policy')
    definition = json.loads(policy_file.read_bytes())
    catalog = Catalog(path, definition['policy'], definition['server'])
    environment = dict(os.environ)
    # The model credential belongs to the Grok/relay hop, never to the MCP child.
    environment.pop(TOKEN, None)
    with subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, env=environment) as peer:
        output_lock = threading.Lock()

        def forward(frame):
            with output_lock:
                send(sys.stdout.buffer, encode(frame) + b'\n')

        def commands():
            try:
                for line in lines(sys.stdin.buffer):
                    msg = json.loads(line)
                    if msg.get('method') == 'tools/call':
                        admitted = catalog.admits(msg.get('params', {}).get('name'))
                        if admitted and sys.platform == 'linux':
                            # A child kept alive by Grok must still observe session rebinding.
                            try:
                                policy = policy_file.open('rb')
                            except FileNotFoundError:
                                admitted = False
                            else:
                                with policy:
                                    fcntl.flock(policy, fcntl.LOCK_SH)
                                    if os.fstat(policy.fileno()).st_nlink:
                                        send(peer.stdin, line)
                                        continue
                                    admitted = False
                        if not admitted:
                            if 'id' in msg:
                                forward({'jsonrpc': '2.0', 'id': msg['id'], 'error': {
                                    'code': -32602, 'message': 'MCP tool is outside the selected policy',
                                }})
                            continue
                    catalog.request(msg)
                    send(peer.stdin, line)
            except (OSError, ValueError):
                pass
            finally:
                peer.stdin.close()

        threading.Thread(target=commands, daemon=True).start()
        try:
            for line in lines(peer.stdout):
                frame = json.loads(line)
                catalog.response(frame)
                forward(frame)
        finally:
            path.unlink(missing_ok=True)
            stop(peer)
    return peer.returncode


def valid_name(value, pattern):
    return isinstance(value, str) and pattern.fullmatch(value) and '__' not in value


def source_hints(cwd):
    if cwd is None:
        return ''
    workspace = Path(cwd).resolve()
    paths = set()
    patterns = ('app/*.ts', 'app/*.tsx', 'app/*/index.ts', 'app/*/index.tsx',
                'packages/*/src/index.ts', 'packages/*/src/index.tsx')
    for pattern in patterns:
        for path in workspace.glob(pattern):
            name = path.relative_to(workspace).as_posix()
            if (SOURCE_PATH.fullmatch(name) and path.is_file()
                    and workspace in path.resolve().parents):
                paths.add(name)
            if len(paths) > 64:
                return ''
    paths = sorted(paths)
    if not paths or len(paths) > 64 or sum(map(len, paths)) > 4096:
        return ''
    return (' Use an existing repository-relative file path. For TypeScript directory imports, '
            'prefer the actual index file over guessing an extension. Existing entry files: '
            + ', '.join(paths) + '.')


def project_request(body, catalogs, cwd=None):
    native = {tool.get('function', {}).get('name') for tool in body.get('tools', [])}
    if USE not in native or catalogs is None:
        return None
    candidates = {}
    counts = {}
    for server, tools in catalogs.items():
        for tool in tools:
            name = tool.get('name')
            if not valid_name(server, SERVER_NAME) or not valid_name(name, TOOL_NAME) or not isinstance(tool.get('inputSchema'), dict):
                continue
            qualified = f'{server}__{name}'
            if '___' in qualified or len(qualified) > 256:
                continue
            if qualified in candidates:
                raise ValueError('duplicate MCP tool name')
            candidates[qualified] = tool
            counts[name] = counts.get(name, 0) + 1

    mapping = {name: None for name in native - DISCOVERY if name}
    projected = [tool for tool in body['tools'] if tool.get('function', {}).get('name') not in DISCOVERY]
    for qualified, tool in candidates.items():
        name = tool['name']
        alias = name if counts[name] == 1 and name not in native and len(name) <= 64 else qualified
        if len(alias) > 64:
            server, _ = qualified.split('__', 1)
            alias = host_tool_id(server, name)
        if not alias or alias in mapping:
            raise ValueError('MCP tool cannot be represented in the model catalog')
        mapping[alias] = qualified
        description = tool.get('description') or ''
        schema = tool['inputSchema']
        if name in {'rustcodegraph_query', 'rustcodegraph_callers', 'rustcodegraph_node'}:
            description += (' Omit projectPath to use the mounted index. projectPath identifies an indexed '
                            'workspace root, never a source file; use rustcodegraph_node.file for files.')
            properties = schema.get('properties', {})
            if 'projectPath' in properties:
                schema = dict(schema, properties=dict(properties, projectPath=dict(
                    properties['projectPath'], description='Indexed workspace root. Omit for this mount.',
                )))
            if name == 'rustcodegraph_node':
                description += source_hints(cwd)
        projected.append({'type': 'function', 'function': {
            'name': alias, 'description': description, 'parameters': schema,
        }})

    if projected:
        body['tools'] = projected
    else:
        for key in ('tools', 'tool_choice', 'parallel_tool_calls'):
            body.pop(key, None)
    messages = body.get('messages', [])
    if messages and messages[0].get('role') == 'system' and isinstance(messages[0].get('content'), str):
        guidance = (
            'The function list in this request is authoritative. Use projected MCP functions directly. '
            'If the user limits tools to a named server, respect that limit even if unavailable; '
            'do not probe with terminal, file, or other tools to check whether it is mounted.'
        )
        messages[0]['content'] += '\n\n' + guidance
    aliases = {qualified: alias for alias, qualified in mapping.items() if qualified}
    for message in messages:
        for call in message.get('tool_calls', []):
            function = call.get('function', {})
            if function.get('name') != USE:
                continue
            args = json.loads(function['arguments'])
            alias = aliases.get(args.get('tool_name'))
            if alias and isinstance(args.get('tool_input'), dict):
                function.update(name=alias, arguments=encode(args['tool_input']).decode())
    return mapping


def project_stream(response, mapping, authorize=lambda _name: True):
    pending = {}
    for line in lines(response):
        if not line.startswith(b'data: ') or line.strip() == b'data: [DONE]':
            if line.strip() == b'data: [DONE]' and pending:
                raise ValueError('unfinished tool invocation')
            yield line
            continue
        chunk = json.loads(line[6:])
        for choice in chunk.get('choices', []):
            delta = choice.get('delta', {})
            index = choice['index']
            for call in delta.pop('tool_calls', []):
                tool = pending.setdefault((index, call['index']), {'id': '', 'name': '', 'arguments': ''})
                identifier = call.get('id')
                if identifier:
                    if tool['id'] and tool['id'] != identifier:
                        raise ValueError('stream changed its tool call ID')
                    tool['id'] = identifier
                for key in ('name', 'arguments'):
                    tool[key] += call.get('function', {}).get(key) or ''
                if sum(map(len, tool.values())) > MAX_FRAME:
                    raise ValueError('tool invocation exceeded the size limit')
            if choice.get('finish_reason') == 'tool_calls':
                calls = []
                for key in sorted(key for key in pending if key[0] == index):
                    tool = pending.pop(key)
                    name = tool['name']
                    if name not in mapping or not tool['id']:
                        raise ValueError('unselected tool invocation')
                    arguments = tool['arguments']
                    if mapping[name] is not None:
                        if not authorize(mapping[name]):
                            raise ValueError('MCP tool was revoked during the request')
                        value = json.loads(arguments)
                        if not isinstance(value, dict):
                            raise ValueError('MCP arguments must be an object')
                        name, arguments = USE, encode({'tool_name': mapping[name], 'tool_input': value}).decode()
                    calls.append({'index': key[1], 'id': tool['id'], 'type': 'function',
                                  'function': {'name': name, 'arguments': arguments}})
                if not calls:
                    raise ValueError('missing streamed tool calls')
                frame = dict(chunk, choices=[dict(choice, delta={'tool_calls': calls}, finish_reason=None)])
                yield b'data: ' + encode(frame) + b'\n\n'
        yield b'data: ' + encode(chunk) + b'\n'
    if pending:
        raise ValueError('unfinished tool invocation')


def tool_policy(server):
    return (server.get('_meta') or {}).get('shiju.grok.toolPolicy') or {}


class Selection:
    def __init__(self, root, script):
        self.root = root
        self.script = script
        self.generation = 0
        self.close()

    def close(self):
        previous = self.snapshot
        self.snapshot = (None, {})
        if sys.platform == 'linux' and previous is not None:
            for path in previous[1].values():
                policy_file = path.with_suffix('.policy')
                with policy_file.open('rb') as policy:
                    fcntl.flock(policy, fcntl.LOCK_EX)
                    policy_file.unlink()

    def bind(self, params):
        if sys.platform == 'linux':
            self.close()
        servers = params.get('mcpServers', [])
        # Remote and mixed transports retain Grok's native discovery path.
        if any(server.get('type', 'stdio') != 'stdio' for server in servers):
            for server in servers:
                policy = tool_policy(server)
                if policy.get('allow') or policy.get('deny'):
                    raise ValueError('native mixed MCP transport cannot enforce scoped stdio tool policy')
            self.snapshot = None
            return
        self.generation += 1
        directory = self.root / f'catalog-{self.generation}'
        directory.mkdir(mode=0o700)
        paths = {}
        for index, server in enumerate(servers):
            name = server['name']
            if not valid_name(name, SERVER_NAME) or name.endswith('_') or name in paths or 'command' not in server:
                raise ValueError('unsupported or ambiguous MCP server name')
            path = directory / f'{index}.json'
            paths[name] = path
            publish(path.with_suffix('.policy'), {'server': name, 'policy': tool_policy(server)})
            server['args'] = ['-I', str(self.script), 'mcp', str(path), server['command'], *server['args']]
            server['command'] = sys.executable
        self.snapshot = (params.get('cwd'), paths)

    def current(self, snapshot):
        if snapshot is None:
            return None, None
        cwd, paths = snapshot
        deadline = time.monotonic() + 5
        while True:
            catalogs = {name: read_catalog(path) for name, path in paths.items()}
            if self.snapshot is not snapshot:
                raise ValueError('ACP MCP selection changed during the request')
            if all(tools is not None for tools in catalogs.values()):
                return cwd, catalogs
            if time.monotonic() >= deadline:
                raise ValueError('native MCP discovery did not complete')
            time.sleep(.025)

    def authorizes(self, snapshot, qualified):
        if self.snapshot is not snapshot:
            return False
        name, tool = qualified.split('__', 1)
        path = snapshot[1].get(name)
        return path is not None and any(
            entry.get('name') == tool for entry in (read_catalog(path) or [])
        )


class Relay(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_POST(self):
        auth = 'Bearer ' + self.server.token
        if (self.path != '/v1/chat/completions'
                or not hmac.compare_digest(self.headers.get('Authorization', ''), auth)):
            self.send_error(403)
            return
        upstream = http.client.HTTPConnection('127.0.0.1', self.server.relay_port, timeout=300)
        started = False
        try:
            length = int(self.headers.get('Content-Length', '0'))
            if not 0 < length <= MAX_FRAME:
                raise ValueError('invalid request length')
            body = json.loads(self.rfile.read(length))
            offered = {tool.get('function', {}).get('name') for tool in body.get('tools', [])}
            mapping = None
            selection = self.server.selection
            snapshot = selection.snapshot
            if USE in offered:
                cwd, catalogs = selection.current(snapshot)
                mapping = project_request(body, catalogs, cwd)
            upstream.request('POST', self.path, body=encode(body), headers={
                'Content-Type': 'application/json', 'Authorization': auth,
            })
            response = upstream.getresponse()
            content_type = response.getheader('Content-Type', 'application/json')
            if mapping is not None and response.status == 200 and 'text/event-stream' not in content_type:
                raise ValueError('expected a streaming response from the model relay')
            self.send_response(response.status)
            self.send_header('Content-Type', content_type)
            self.end_headers()
            started = True
            if mapping is not None and response.status == 200:
                output = project_stream(response, mapping, lambda tool: selection.authorizes(snapshot, tool))
            else:
                output = lines(response)
            for line in output:
                send(self.wfile, line)
        except (OSError, ValueError, KeyError, TypeError, http.client.HTTPException):
            if not started:
                self.send_error(502, 'Grok relay projection failed')
            elif mapping is not None:
                try:
                    send(self.wfile, b'data: {"error":{"message":"Grok relay projection failed"}}\n\n')
                except OSError:
                    pass
        finally:
            upstream.close()


def stop(proc):
    try:
        proc.wait(timeout=2)
    except subprocess.TimeoutExpired:
        proc.terminate()
        try:
            proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()


def launch(script, config, port, command):
    home = Path(os.environ['GROK_HOME'])
    lock_fd = os.open(home / 'shiju-projection.lock', os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    with os.fdopen(lock_fd, 'wb') as lock, ThreadingHTTPServer(('127.0.0.1', 0), Relay) as server:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        server.token = os.environ[TOKEN]
        server.relay_port = int(port)
        server.selection = Selection(config.parent, script)
        original = f'http://127.0.0.1:{port}/v1'
        contents = config.read_text()
        if original not in contents:
            raise ValueError('missing private relay in Grok configuration')
        publish(home / 'config.toml', contents.replace(
            original, f'http://127.0.0.1:{server.server_port}/v1').encode())
        threading.Thread(target=server.serve_forever, daemon=True).start()
        with subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE) as proc:
            def requests():
                try:
                    for line in lines(sys.stdin.buffer):
                        msg = json.loads(line)
                        method = msg.get('method')
                        if method in SESSIONS:
                            server.selection.bind(msg['params'])
                            line = encode(msg) + b'\n'
                        elif method == 'session/close':
                            server.selection.close()
                        send(proc.stdin, line)
                except (OSError, ValueError, KeyError, TypeError):
                    pass
                finally:
                    try:
                        if sys.platform == 'linux':
                            server.selection.close()
                    finally:
                        proc.stdin.close()

            threading.Thread(target=requests, daemon=True).start()
            try:
                for line in lines(proc.stdout):
                    send(sys.stdout.buffer, line)
            finally:
                stop(proc)
                server.shutdown()
        return proc.returncode


if __name__ == '__main__':
    try:
        if sys.argv[1] == 'mcp':
            code = forward_mcp(Path(sys.argv[2]), sys.argv[3:])
        else:
            code = launch(Path(__file__), Path(sys.argv[1]), sys.argv[2], sys.argv[3:])
    except (OSError, ValueError, KeyError, TypeError):
        sys.stderr.write('Grok managed projection unavailable\n')
        code = 1
    sys.exit(code)
