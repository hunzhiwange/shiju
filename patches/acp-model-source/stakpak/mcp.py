"""Adapt the reviewed Stakpak startup catalog to one host-owned ACP session."""

import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys

from mcp_bridge import Mounts, POLICY, arguments_key


BUILTIN_NAMES = ('stakpak__view', 'stakpak__create', 'stakpak__str_replace')
BUILTIN_TOOLS = 'allowed_tools = ' + json.dumps(BUILTIN_NAMES)
BUILTIN_TITLES = {'Read', 'Creating', 'Editing'}
RESERVED_MCP_TOOLS = {'view', 'create', 'create_file', 'str_replace', 'edit_file'}
POLICY_KEY = 'shiju.stakpak.toolPolicy'
LIMIT = 4 * 1024 * 1024


def send(stream, frame):
    stream.write(json.dumps(frame, separators=(',', ':'), allow_nan=False).encode() + b'\n')
    stream.flush()


def read(stream):
    line = stream.readline(LIMIT + 1)
    if not line or len(line) > LIMIT:
        raise EOFError
    return json.loads(line)


def response(identifier, **fields):
    return dict(jsonrpc='2.0', id=identifier, **fields)


def fail(identifier, message):
    send(sys.stdout.buffer, response(identifier, error=dict(code=-32000, message=message)))


def stop(child):
    if child.poll() is None:
        child.terminate()
        try:
            child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait()
    child.stdin.close()
    child.stdout.close()


def initialize(child, frame):
    send(child.stdin, frame)
    with selectors.DefaultSelector() as selector:
        selector.register(child.stdout, selectors.EVENT_READ)
        if not selector.select(timeout=15):
            raise ValueError('Stakpak native initialization timed out')
    reply = read(child.stdout)
    if reply.get('id') != frame['id'] or reply.get('method'):
        raise ValueError('Stakpak native initialization response is invalid')
    return reply


def native_title(tool):
    return ' '.join(word.capitalize() for word in tool.split('_'))


class StakpakMounts(Mounts):
    def __init__(self, workspace, root):
        super().__init__(workspace, root / 'mcp-bridge.json')
        self.root = root
        self.titles = {}
        self.mount_config = None

    def __exit__(self, *args):
        try:
            super().__exit__(*args)
        finally:
            if self.mount_config is not None:
                self.mount_config.unlink(missing_ok=True)
            self.settings.unlink(missing_ok=True)

    def prepare(self, definitions, config):
        self.settings.write_text('{}')
        config_dir = self.root / '.stakpak'
        config_path = config_dir / 'config.toml'
        if config.count(BUILTIN_TOOLS) != 1:
            raise ValueError('Stakpak MCP requires the reviewed private tool list')
        selected = []
        for definition in definitions:
            policy = (definition.get('_meta') or {}).get(POLICY_KEY)
            selected.append({**definition, '_meta': {POLICY: policy}})
        self.install(selected)
        entries = {}
        tools = list(BUILTIN_NAMES)
        for server, catalog in self.catalogs.items():
            entries[server] = dict(url=f'http://127.0.0.1:{self.http.server_port}/{server}',
                                   headers={'Authorization': 'Bearer ' + self.tokens[server]})
            for tool in catalog:
                title = native_title(tool)
                if (title in self.titles or title in BUILTIN_TITLES
                        or tool.lower() in RESERVED_MCP_TOOLS):
                    raise ValueError('Stakpak MCP tool titles must be unambiguous')
                self.titles[title] = (server, tool)
                tools.append(server + '__' + tool)
        if not self.titles:
            raise ValueError('Stakpak MCP selection has no permitted tools')
        config_file = config_dir / 'mcp.json'
        with config_file.open('x') as output:
            self.mount_config = config_file
            json.dump({'mcpServers': entries}, output)
        self.settings.unlink()
        config_path.write_text(config.replace(BUILTIN_TOOLS, 'allowed_tools = ' + json.dumps(tools)))

    def observe(self, session, update):
        if update.get('sessionUpdate') != 'tool_call':
            return
        with self.lock:
            if self.turn == session and update.get('title') and update.get('toolCallId'):
                self.calls[update['toolCallId']] = (update['title'], update.get('rawInput'))

    def permission(self, identifier, params):
        call = params.get('toolCall', {})
        arguments = call.get('rawInput')
        title = call.get('title')
        if isinstance(arguments, dict) and isinstance(arguments.get('path'), str):
            suffix = ' ' + arguments['path']
            if isinstance(title, str) and title.endswith(suffix):
                title = title[:-len(suffix)]
        pair = self.titles.get(title)
        options = params.get('options', [])
        allow = [option for option in options if option.get('kind') == 'allow_once']
        with self.lock:
            observed = self.calls.pop(call.get('toolCallId'), None)
            displayed = {**arguments} if isinstance(arguments, dict) else {}
            for key in ('old_str', 'new_str'):
                if key in displayed:
                    displayed[key + 'ing'] = displayed.pop(key)
            if (observed is None or observed[0] != call.get('title') or not isinstance(observed[1], dict)
                    or arguments_key({**observed[1], 'abs_path': displayed.get('abs_path')})
                    != arguments_key({**displayed, 'abs_path': displayed.get('abs_path')})):
                return False
            if (not isinstance(arguments, dict) or self.turn is None
                    or self.turn != params.get('sessionId') or identifier in self.pending):
                return False
            if pair is None:
                # Native tools retain their existing ACP permission and filesystem paths.
                return title in BUILTIN_TITLES
            if len(allow) != 1:
                return False
            self.pending[identifier] = (allow[0]['optionId'], (*pair, arguments_key(arguments)))
            params['options'] = allow + [option for option in options if option.get('kind') == 'reject_once']
            return True


def frames(child, host):
    buffers = {'host': bytearray(), 'native': bytearray()}
    with selectors.DefaultSelector() as selector:
        selector.register(host, selectors.EVENT_READ, 'host')
        selector.register(child.stdout, selectors.EVENT_READ, 'native')
        while True:
            for key, _ in selector.select():
                data = os.read(key.fd, 65536)
                if not data:
                    return
                buffer = buffers[key.data]
                buffer.extend(data)
                lines = buffer.split(b'\n')
                buffers[key.data] = lines.pop()
                if len(buffers[key.data]) > LIMIT or any(len(line) > LIMIT for line in lines):
                    raise ValueError('Stakpak ACP frame exceeds limit')
                for line in lines:
                    yield key.data, json.loads(line)


def relay(child, mounts, initial_request, host):
    new_id = initial_request['id'] if mounts.http is not None else None
    native_session = None
    prompt_id = None
    send(child.stdin, initial_request)
    for source, frame in frames(child, host):
        identifier, method = frame.get('id'), frame.get('method')
        params = frame.get('params', {})
        if mounts.http is not None:
            if source == 'host':
                if method is None:
                    mounts.decide(identifier, frame.get('result'))
                elif method in {'session/new', 'session/load', 'session/resume', 'session/fork'}:
                    fail(identifier, 'Stakpak MCP is bound to the new session')
                    continue
                elif method == 'session/prompt':
                    if native_session is None or params.get('sessionId') != native_session or prompt_id is not None:
                        fail(identifier, 'Stakpak MCP prompt must own the mounted session')
                        continue
                    prompt_id = identifier
                    mounts.begin(native_session)
                elif method in {'session/cancel', 'session/close'} and params.get('sessionId') == native_session:
                    mounts.end()
                    if method == 'session/close':
                        mounts.revoke()
                        native_session = None
            else:
                if method == 'session/update':
                    mounts.observe(params.get('sessionId'), params.get('update', {}))
                if method == 'session/request_permission':
                    if not mounts.permission(identifier, params):
                        send(child.stdin, response(identifier, result={'outcome': {'outcome': 'cancelled'}}))
                        continue
                elif method is None and identifier is not None:
                    if identifier == new_id:
                        new_id = None
                        native_session = frame.get('result', {}).get('sessionId')
                        if not native_session:
                            mounts.revoke()
                    if identifier == prompt_id:
                        prompt_id = None
                        mounts.end()
        send(child.stdin if source == 'host' else sys.stdout.buffer, frame)


def run(command, env, root):
    workspace = Path.cwd().resolve()
    for name in ('mcp.toml', 'mcp.json', '.stakpak/mcp.toml', '.stakpak/mcp.json'):
        if os.path.lexists(workspace / name):
            raise ValueError('Stakpak project MCP overrides are not verified')
    for name in ('mcp.toml', 'mcp.json'):
        if os.path.lexists(root / '.stakpak' / name):
            raise ValueError('Stakpak private runtime already has an MCP configuration')
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, terminate)

    def start():
        return subprocess.Popen(command, env=env, stdin=subprocess.PIPE,
                                stdout=subprocess.PIPE, bufsize=0)

    # Use raw stdin so reading the initial frames does not hide bytes from selectors.
    initial = sys.stdin.buffer.raw
    hello = read(initial)
    if hello.get('method') != 'initialize' or 'id' not in hello:
        raise ValueError('Stakpak requires ACP initialize before the session')
    config = (root / '.stakpak/config.toml').read_text()
    child = start()
    with StakpakMounts(workspace, root) as mounts:
        try:
            reply = initialize(child, hello)
            send(sys.stdout.buffer, reply)
            if 'result' not in reply:
                return
            while True:
                first = read(initial)
                method = first.get('method')
                if method in {'session/new', 'session/load', 'session/resume'}:
                    definitions = first.get('params', {}).get('mcpServers', [])
                    if definitions:
                        try:
                            if (method != 'session/new'
                                    or Path(first['params']['cwd']).resolve(strict=True) != workspace
                                    or reply['result'].get('authMethods')):
                                raise ValueError('Stakpak MCP requires a new session in the launch workspace')
                            # The fixed native release snapshots tools at process startup.
                            # Replay the real handshake in a fresh process before creating a session.
                            stop(child)
                            mounts.prepare(definitions, config)
                            child = start()
                            if initialize(child, hello) != reply:
                                raise ValueError('Stakpak initialization changed on mount')
                            first['params']['mcpServers'] = []
                        except (EOFError, KeyError, OSError, TypeError, ValueError, RuntimeError, TimeoutError):
                            fail(first['id'], 'Stakpak could not mount the selected MCP tools')
                            return
                    relay(child, mounts, first, initial)
                    return
                if 'id' in first:
                    fail(first['id'], 'Stakpak requires session setup after initialization')
        finally:
            mounts.end()
            stop(child)


def terminate(signum, _frame):
    raise SystemExit(128 + signum)
