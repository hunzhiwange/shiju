"""Translate explicit ACP mounts to Cortex client tools without changing its sandbox."""
import asyncio
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import sys
import tempfile
import uuid

LIMIT = 4 * 1024 * 1024
TIMEOUT = 15
PREFIX = 'shiju_mcp_'
APPROVAL_PREFIX = 'shiju-cortex-approval-'
NAME = re.compile(r'[a-zA-Z0-9]+(?:_[a-zA-Z0-9]+)*')
MCP_SANDBOX = """
(version 1)
(allow default)
(deny file-read-data (subpath "/Users") (subpath "/private/tmp")
    (subpath "/private/var/folders") (subpath "/Volumes") (subpath (param "host-home")))
(allow file-read-data (literal (param "command")) (subpath (param "workspace"))
    (subpath (param "scratch")))
(deny file-read-data (subpath (param "git")) (subpath (param "project-settings"))
    (literal (param "env-file")) (literal (param "mount-config")))
(deny file-write*)
(allow file-write* (subpath (param "scratch")) (literal "/dev/null"))
(deny process-fork process-exec network*)
(allow process-exec (literal (param "command")))
"""


def encode(message):
    return json.dumps({'jsonrpc': '2.0', **message}, separators=(',', ':')).encode() + b'\n'


def fail(identifier, message):
    return {'id': identifier, 'error': {'code': -32603, 'message': message}}


def allowed_tool(definition, server, name):
    policy = (definition.get('_meta') or {}).get('shiju.cortex.toolPolicy', {})
    allow, deny = policy.get('allow', []), policy.get('deny', [])
    tool_id = f'mcp__{server.lower()}__{name.lower()}'
    if server != server.lower() or name != name.lower() or len(tool_id) > 64:
        digest = hashlib.sha256(f'{server}\0{name}'.encode()).hexdigest()[:16]
        tool_id = f'{tool_id[:47]}_{digest}'
    matches = {name, tool_id}
    return not matches.intersection(deny) and (not allow or matches.intersection(allow))


class Stdio:
    def __init__(self, definition, workspace):
        command = Path(definition.get('command', ''))
        args = definition.get('args', [])
        if (definition.get('type') not in (None, 'stdio') or not command.is_absolute()
                or not command.is_file() or not isinstance(args, list)
                or any(not isinstance(arg, str) for arg in args) or definition.get('env')):
            raise ValueError('Cortex MCP requires an absolute stdio command and empty environment')
        self.command = [str(command.resolve(strict=True)), *args]
        self.workspace = workspace
        self.process = None
        self.scratch = None
        self.lock = asyncio.Lock()

    async def close(self):
        process, self.process = self.process, None
        if process:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            await process.wait()
        if self.scratch:
            self.scratch.cleanup()
            self.scratch = None

    async def request(self, method, params):
        identifier = uuid.uuid4().hex
        self.process.stdin.write(encode({'id': identifier, 'method': method, 'params': params}))
        await self.process.stdin.drain()
        while line := await self.process.stdout.readline():
            reply = json.loads(line)
            if reply.get('id') == identifier and 'method' not in reply:
                if 'error' in reply or not isinstance(reply.get('result'), dict):
                    raise ValueError('MCP server rejected request')
                return reply['result']
            if 'method' in reply and 'id' in reply:
                self.process.stdin.write(encode(fail(reply['id'], 'Unsupported MCP callback')))
                await self.process.stdin.drain()
        raise ValueError('MCP server disconnected')

    async def start(self):
        if self.process is None:
            self.scratch = tempfile.TemporaryDirectory(prefix='shiju-cortex-mcp-')
            scratch = Path(self.scratch.name).resolve()
            sandbox = ['/usr/bin/sandbox-exec']
            for key, value in {
                'command': self.command[0], 'workspace': self.workspace, 'scratch': scratch,
                'host-home': Path.home().resolve(strict=True),
                'git': self.workspace / '.git', 'project-settings': self.workspace / '.cortex',
                'env-file': self.workspace / '.env', 'mount-config': self.workspace / '.vwacprc.json',
            }.items():
                sandbox.extend(['-D', f'{key}={value}'])
            self.process = await asyncio.create_subprocess_exec(
                *sandbox, '-p', MCP_SANDBOX, *self.command, cwd=self.workspace,
                env={'HOME': str(scratch), 'TMPDIR': str(scratch), 'LANG': 'en_US.UTF-8'},
                stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.DEVNULL, start_new_session=True, limit=LIMIT,
            )
            result = await self.request('initialize', {
                'protocolVersion': '2024-11-05', 'capabilities': {},
                'clientInfo': {'name': 'shiju-cortex', 'version': '1'},
            })
            if 'tools' not in result.get('capabilities', {}):
                raise ValueError('MCP server does not advertise tools')
            self.process.stdin.write(encode({'method': 'notifications/initialized'}))
            await self.process.stdin.drain()

    async def list_tools(self):
        await self.start()
        cursor = None
        for _ in range(16):
            page = await self.request('tools/list', {'cursor': cursor} if cursor else {})
            for tool in page['tools']:
                yield tool
            cursor = page.get('nextCursor')
            if not cursor:
                return
        raise ValueError('Cortex MCP tool catalog is too large')

    async def call(self, name, arguments):
        async with self.lock:
            try:
                await self.start()
                return await self.request('tools/call', {'name': name, 'arguments': arguments})
            except (Exception, asyncio.CancelledError):
                await self.close()
                raise


class Bridge:
    def __init__(self, native, workspace):
        self.native = native
        self.workspace = workspace
        self.sessions = {}
        self.setup = None
        self.turn = None
        self.cancelled = False
        self.calls = {}
        self.pending = {}
        self.workers = set()

    def host(self, message):
        sys.stdout.buffer.write(encode(message))
        sys.stdout.buffer.flush()

    async def send(self, message):
        self.native.stdin.write(encode(message))
        await self.native.stdin.drain()

    async def stop(self, selection=None):
        peers = set(selection[1] for selection in (selection or {}).values())
        await asyncio.gather(*(peer.close() for peer in peers))

    async def selection(self, definitions):
        selected = {}
        services = {}
        try:
            for definition in definitions:
                server = definition.get('name', '')
                if not NAME.fullmatch(server) or server in services:
                    raise ValueError('Invalid or duplicate Cortex MCP service name')
                services[server] = peer = Stdio(definition, self.workspace)
                async for tool in peer.list_tools():
                    name = tool['name']
                    public = PREFIX + server + '__' + name
                    if not NAME.fullmatch(name):
                        raise ValueError('Invalid Cortex MCP tool name')
                    if not allowed_tool(definition, server, name):
                        continue
                    if (len(public) > 64 or public in selected
                            or len(selected) >= 256 or not isinstance(tool.get('inputSchema'), dict)):
                        raise ValueError('Invalid or duplicate Cortex MCP tool name')
                    selected[public] = (name, peer, {
                        'name': public, 'description': tool.get('description', name),
                        'inputSchema': tool['inputSchema'],
                    })
            return selected
        finally:
            # Discovery never leaves a subprocess running while ACP waits for a prompt.
            await asyncio.gather(*(peer.close() for peer in services.values()))

    async def from_host(self, incoming):
        async for line in lines(incoming):
            message = json.loads(line)
            method, identifier = message.get('method'), message.get('id')
            params = message.get('params', {})
            session = params.get('sessionId')
            if not method and isinstance(identifier, str) and identifier.startswith(APPROVAL_PREFIX):
                waiter = self.pending.pop(identifier, None)
                if waiter and not waiter.done():
                    waiter.set_result(message.get('result', {}))
                continue
            if method == 'session/fork':
                if self.turn or self.setup or self.sessions.get(session):
                    self.host(fail(identifier, 'Cortex MCP sessions cannot be forked'))
                    continue
                self.setup = (identifier, None, {})
                params['_meta'] = {**(params.get('_meta') or {}), 'clientTools': []}
            if method in ('session/new', 'session/load'):
                if self.turn or self.setup:
                    self.host(fail(identifier, 'Cortex session is busy'))
                    continue
                try:
                    if Path(params['cwd']).resolve(strict=True) != self.workspace:
                        raise ValueError('Cortex workspace mismatch')
                    selected = await asyncio.wait_for(self.selection(params.get('mcpServers', [])), TIMEOUT)
                    params['mcpServers'] = []
                except Exception:
                    self.host(fail(identifier, 'Could not discover explicit Cortex MCP tools'))
                    continue
                self.setup = (identifier, session, selected)
                params['_meta'] = {**(params.get('_meta') or {}), 'clientTools': [
                    entry[2] for entry in selected.values()
                ]}
            if method == 'session/prompt':
                if self.turn or self.setup or session not in self.sessions:
                    self.host(fail(identifier, 'Cortex prompt has no idle session'))
                    continue
                self.turn = (identifier, session)
                self.cancelled = False
            if method in ('session/cancel', 'session/close') and self.turn and session == self.turn[1]:
                self.cancelled = True
                await self.cancel_calls()
            if method == 'session/close':
                await self.stop(self.sessions.pop(session, None))
            await self.send(message)

    async def cancel_calls(self):
        self.calls.clear()
        running = list(self.workers)
        for task in running:
            task.cancel()
        await asyncio.gather(*running, return_exceptions=True)

    async def from_native(self):
        async for line in lines(self.native.stdout):
            message = json.loads(line)
            identifier, method = message.get('id'), message.get('method')
            params = message.get('params', {})
            if self.setup and not method and identifier == self.setup[0]:
                _, restored, selected = self.setup
                self.setup = None
                if 'result' in message:
                    session = message['result'].get('sessionId', restored)
                    self.sessions.clear()
                    self.sessions[session] = selected
            if self.turn and not method and identifier == self.turn[0]:
                session = self.turn[1]
                await self.cancel_calls()
                await self.stop(self.sessions.get(session))
                self.turn = None
            if method == 'session/update' and self.turn and params.get('sessionId') == self.turn[1]:
                update = params.get('update', {})
                title = update.get('title', '')
                if update.get('sessionUpdate') == 'tool_call' and title in self.sessions.get(self.turn[1], {}):
                    self.calls[update['toolCallId']] = (title, update.get('rawInput'))
            if identifier is not None and method and method.startswith(PREFIX):
                task = asyncio.create_task(self.call(identifier, method, params))
                self.workers.add(task)
                task.add_done_callback(self.workers.discard)
            else:
                self.host(message)

    async def call(self, identifier, tool, arguments):
        result = {'error': 'Cortex MCP call has no approved active tool'}
        turn = self.turn
        approval = APPROVAL_PREFIX + uuid.uuid4().hex
        try:
            selection = self.sessions.get(turn[1], {}) if turn else {}
            call_id = next((key for key, value in self.calls.items() if value == (tool, arguments)), None)
            if not self.cancelled and call_id is not None and tool in selection and isinstance(arguments, dict):
                self.calls.pop(call_id)
                waiter = asyncio.get_running_loop().create_future()
                self.pending[approval] = waiter
                self.host({'id': approval, 'method': 'session/request_permission', 'params': {
                    'sessionId': turn[1],
                    'toolCall': {'toolCallId': call_id, 'title': tool, 'kind': 'other', 'rawInput': arguments},
                    'options': [
                        {'optionId': 'allow_once', 'kind': 'allow_once', 'name': 'Allow once'},
                        {'optionId': 'reject_once', 'kind': 'reject_once', 'name': 'Reject'},
                    ],
                }})
                decision = (await waiter).get('outcome', {})
                if (not self.cancelled and self.turn == turn
                        and decision == {'outcome': 'selected', 'optionId': 'allow_once'}):
                    name, peer, _ = selection[tool]
                    reply = await asyncio.wait_for(peer.call(name, arguments), TIMEOUT)
                    result = {'error': 'Cortex MCP tool returned an error'} if reply.get('isError') else reply
        except asyncio.CancelledError:
            result = {'error': 'Cortex MCP call cancelled'}
        except Exception:
            result = {'error': 'Cortex MCP tool execution failed'}
        finally:
            self.pending.pop(approval, None)
        await self.send({'id': identifier, 'result': result})


async def lines(reader):
    while line := await reader.readline():
        yield line


async def run(command, env, workspace):
    loop = asyncio.get_running_loop()
    reader = asyncio.StreamReader(limit=LIMIT)
    await loop.connect_read_pipe(lambda: asyncio.StreamReaderProtocol(reader), sys.stdin.buffer)
    native = await asyncio.create_subprocess_exec(
        *command, env=env, stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE,
        limit=LIMIT,
    )
    bridge = Bridge(native, workspace)
    stop = asyncio.Event()
    for sig in (signal.SIGINT, signal.SIGTERM):
        loop.add_signal_handler(sig, stop.set)
    peer = asyncio.create_task(bridge.from_native())
    tasks = {peer, asyncio.create_task(bridge.from_host(reader)), asyncio.create_task(stop.wait())}
    status = 0
    try:
        done, _ = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
        for task in done:
            task.result()
        if peer in done:
            status = await native.wait()
    finally:
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)
        await bridge.cancel_calls()
        for selection in bridge.sessions.values():
            await bridge.stop(selection)
        if native.returncode is None:
            native.kill()
        await native.wait()
    return status
