"""Expose selected stdio MCP peers through the Junie launch's single loopback socket."""
import asyncio
import concurrent.futures
import hmac
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import re
import secrets
import signal
import sys
import threading

LIMIT = 4 * 1024 * 1024
TIMEOUT = 15
NAME = re.compile(r'[a-zA-Z0-9]+(?:_[a-zA-Z0-9]+)*')
CANCELLED = {'outcome': {'outcome': 'cancelled'}}


def arguments_key(arguments):
    return json.dumps(arguments, sort_keys=True, separators=(',', ':'), allow_nan=False)


class Mount:
    def __init__(self, definition, workspace, stdio, allowed_tool):
        self.name = definition['name']
        self.path = '/mcp/' + secrets.token_urlsafe(32)
        self.peer = stdio(definition, workspace)
        policy = (definition.get('_meta') or {}).get('shiju.junie.toolPolicy', {})
        self.policy = {'_meta': {'shiju.cortex.toolPolicy': policy}}
        self.allowed_tool = allowed_tool
        self.tools = {}
        self.initialized = False

    async def discover(self):
        async for tool in self.peer.list_tools():
            name = tool['name']
            if NAME.fullmatch(name) and self.allowed_tool(self.policy, self.name, name):
                if name in self.tools or len(self.tools) >= 256:
                    raise ValueError('Invalid Junie MCP catalog')
                self.tools[name] = tool


class Session:
    def __init__(self, workspace, stdio, allowed_tool):
        self.workspace = workspace
        self.stdio = stdio
        self.allowed_tool = allowed_tool
        self.mounts = {}
        self.session = None
        self.setup = None
        self.turn = None
        self.cancelled = False
        self.pending = {}
        self.approved = {}
        self.calls = set()

    async def revoke(self):
        self.cancelled = True
        self.pending.clear()
        self.approved.clear()
        calls = list(self.calls)
        for call in calls:
            call.cancel()
        await asyncio.gather(*calls, return_exceptions=True)

    async def clear(self):
        await self.revoke()
        self.turn = None
        self.session = None
        mounts, self.mounts = self.mounts, {}
        await asyncio.gather(*(mount.peer.close() for mount in mounts.values()))

    async def stage(self, message, port):
        params = message['params']
        if Path(params['cwd']).resolve(strict=True) != self.workspace:
            raise ValueError('Junie workspace mismatch')
        servers = params.get('mcpServers', [])
        if not isinstance(servers, list) or len(servers) > 16:
            raise ValueError('Invalid Junie MCP selection')
        names = set()
        rewritten = []
        for definition in servers:
            name = definition['name']
            if not NAME.fullmatch(name) or name in names:
                raise ValueError('Invalid Junie MCP name')
            names.add(name)
            mount = Mount(definition, self.workspace, self.stdio, self.allowed_tool)
            self.mounts[mount.path] = mount
            await mount.discover()
            rewritten.append({
                'type': 'http', 'name': name,
                'url': f'http://127.0.0.1:{port}{mount.path}', 'headers': [],
            })
        params['mcpServers'] = rewritten

    def approval(self, message):
        params = message['params']
        call = params['toolCall']
        if not (call.get('_meta') or {}).get('is_mcp_tool_call'):
            return
        title = call.get('title', '')
        contents = call.get('content') or []
        identifier = message['id']
        choices = [option for option in params['options']
                   if option['kind'] in ('allow_once', 'reject_once')]
        params['options'] = choices
        denies = {option['optionId'] for option in choices if option['kind'] == 'reject_once'}
        for path, mount in self.mounts.items():
            prefix = mount.name + '/'
            if not title.startswith(prefix):
                continue
            name = title[len(prefix):]
            if (self.turn is None or self.cancelled or params['sessionId'] != self.session
                    or name not in mount.tools or len(contents) != 1):
                break
            try:
                arguments = json.loads(contents[0]['content']['text'])
                key = arguments_key(arguments)
            except (ValueError, TypeError, KeyError):
                break
            if not isinstance(arguments, dict):
                break
            allows = {option['optionId'] for option in choices if option['kind'] == 'allow_once'}
            self.pending[identifier] = (allows, denies, (path, name, key))
            return
        self.pending[identifier] = (set(), denies, None)

    def decision(self, message):
        pending = self.pending.pop(message.get('id'), None)
        if pending is None:
            return
        allows, denies, key = pending
        outcome = (message.get('result') or {}).get('outcome', {})
        if outcome.get('outcome') != 'selected':
            return
        option = outcome.get('optionId')
        if option in allows and self.turn is not None and not self.cancelled:
            self.approved[key] = self.approved.get(key, 0) + 1
        elif option not in denies:
            message['result'] = CANCELLED

    async def mcp(self, path, request):
        mount = self.mounts.get(path)
        if mount is None:
            return 404, None
        method = request.get('method')
        identifier = request.get('id')
        if method == 'initialize' and identifier is not None:
            result = {'protocolVersion': '2024-11-05', 'capabilities': {'tools': {}},
                      'serverInfo': {'name': mount.name, 'version': '1'}}
        elif method == 'notifications/initialized':
            mount.initialized = True
            return 202, None
        elif method == 'tools/list' and mount.initialized:
            result = {'tools': list(mount.tools.values())}
        elif method == 'tools/call' and mount.initialized:
            params = request['params']
            name, arguments = params['name'], params.get('arguments', {})
            key = (path, name, arguments_key(arguments))
            if (identifier is None or self.turn is None or self.cancelled or name not in mount.tools
                    or not isinstance(arguments, dict) or not self.approved.get(key)):
                return 200, {'jsonrpc': '2.0', 'id': identifier, 'error': {
                    'code': -32602, 'message': 'Junie MCP call lacks current approval'}}
            if self.approved[key] == 1:
                del self.approved[key]
            else:
                self.approved[key] -= 1
            task = asyncio.current_task()
            self.calls.add(task)
            try:
                result = await asyncio.wait_for(mount.peer.call(name, arguments), TIMEOUT)
            finally:
                self.calls.discard(task)
        else:
            return 200, {'jsonrpc': '2.0', 'id': identifier, 'error': {
                'code': -32601, 'message': 'Unsupported Junie MCP request'}}
        return 200, {'jsonrpc': '2.0', 'id': identifier, 'result': result}


def handler(loop, session, endpoint, token):
    class Handler(BaseHTTPRequestHandler):
        def setup(self):
            super().setup()
            self.connection.settimeout(TIMEOUT)

        def log_message(self, *_args):
            pass

        def do_GET(self):
            self.send_response(405)
            self.end_headers()

        def do_POST(self):
            try:
                size = int(self.headers.get('Content-Length', '0'))
                if not 0 < size <= LIMIT:
                    self.send_error(413)
                    return
                body = self.rfile.read(size)
                if self.path == '/v1/chat/completions':
                    self.model(body)
                    return
                if not re.fullmatch(r'/mcp/[A-Za-z0-9_-]{43}', self.path):
                    self.send_error(404)
                    return
                request = json.loads(body)
                if not isinstance(request, dict) or request.get('jsonrpc') != '2.0':
                    self.send_error(400)
                    return
                future = asyncio.run_coroutine_threadsafe(session.mcp(self.path, request), loop)
                try:
                    status, response = future.result(TIMEOUT + 2)
                except concurrent.futures.TimeoutError:
                    future.cancel()
                    self.send_error(504)
                    return
                except (ValueError, TypeError, KeyError, OSError):
                    status, response = 200, {'jsonrpc': '2.0', 'id': request.get('id'), 'error': {
                        'code': -32000, 'message': 'Junie MCP request failed'}}
                payload = json.dumps(response, allow_nan=False).encode() if response is not None else b''
                if len(payload) > LIMIT:
                    self.send_error(500)
                    return
                self.send_response(status)
                self.send_header('Content-Length', str(len(payload)))
                if response is not None:
                    self.send_header('Content-Type', 'application/json')
                self.end_headers()
                if payload:
                    self.wfile.write(payload)
            except (OSError, ValueError, TypeError, KeyError, concurrent.futures.CancelledError):
                self.close_connection = True

        def model(self, body):
            provided = self.headers.get('Authorization', '').encode()
            if not hmac.compare_digest(provided, ('Bearer ' + token).encode()):
                self.send_error(401)
                return
            connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=180)
            try:
                connection.request('POST', self.path, body, {
                    'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token,
                })
                response = connection.getresponse()
                self.send_response(response.status)
                self.send_header('Content-Type', response.getheader('Content-Type', 'application/json'))
                self.end_headers()
                while chunk := response.read1(65536):
                    self.wfile.write(chunk)
            finally:
                connection.close()

    return Handler


async def run(command, environment, workspace, endpoint, token, configure, stdio, allowed_tool):
    loop = asyncio.get_running_loop()
    session = Session(workspace, stdio, allowed_tool)
    server = ThreadingHTTPServer(('127.0.0.1', 0), handler(loop, session, endpoint, token))
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    native = None
    tasks = []
    try:
        port = server.server_port
        configure(port)
        native = await asyncio.create_subprocess_exec(
            *command(port), env=environment, stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE, limit=LIMIT,
        )
        host = asyncio.StreamReader(limit=LIMIT)
        await loop.connect_read_pipe(lambda: asyncio.StreamReaderProtocol(host), sys.stdin.buffer)

        async def send(stream, message):
            stream.write((json.dumps(message, separators=(',', ':')) + '\n').encode())
            await stream.drain()

        def emit(message):
            sys.stdout.buffer.write((json.dumps(message, separators=(',', ':')) + '\n').encode())
            sys.stdout.buffer.flush()

        async def from_host():
            while line := await host.readline():
                message = json.loads(line)
                method = message.get('method')
                identifier = message.get('id')
                params = message.get('params', {})
                if method in ('session/new', 'session/load', 'session/resume'):
                    if session.turn is not None or session.setup is not None:
                        emit({'jsonrpc': '2.0', 'id': identifier, 'error': {
                            'code': -32603, 'message': 'Junie session is busy'}})
                        continue
                    try:
                        await session.clear()
                        session.setup = (identifier, params.get('sessionId'))
                        await asyncio.wait_for(session.stage(message, port), TIMEOUT)
                    except Exception:
                        session.setup = None
                        await session.clear()
                        emit({'jsonrpc': '2.0', 'id': identifier, 'error': {
                            'code': -32603, 'message': 'Could not bind Junie session MCP tools'}})
                        continue
                elif method == 'session/prompt' and params.get('sessionId') == session.session:
                    if session.turn is not None:
                        emit({'jsonrpc': '2.0', 'id': identifier, 'error': {
                            'code': -32603, 'message': 'Junie session is busy'}})
                        continue
                    session.turn = identifier
                    session.cancelled = False
                elif method in ('session/cancel', 'session/close') and params.get('sessionId') == session.session:
                    await session.revoke()
                    if method == 'session/close':
                        await session.clear()
                elif not method:
                    session.decision(message)
                await send(native.stdin, message)

        async def from_native():
            while line := await native.stdout.readline():
                message = json.loads(line)
                identifier = message.get('id')
                method = message.get('method')
                if not method:
                    if session.setup is not None and identifier == session.setup[0]:
                        _, restored = session.setup
                        session.setup = None
                        if 'result' in message:
                            session.session = message['result'].get('sessionId', restored)
                        else:
                            await session.clear()
                    if session.turn is not None and identifier == session.turn:
                        await session.revoke()
                        session.turn = None
                elif method == 'session/request_permission':
                    session.approval(message)
                emit(message)

        stopped = asyncio.Event()
        for sig in (signal.SIGINT, signal.SIGTERM):
            loop.add_signal_handler(sig, stopped.set)
        peer = asyncio.create_task(from_native())
        tasks = [peer, asyncio.create_task(from_host()), asyncio.create_task(stopped.wait())]
        done, _ = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
        for task in done:
            task.result()
        return await native.wait() if peer in done else 0
    finally:
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)
        await session.clear()
        if native is not None:
            if native.returncode is None:
                native.kill()
            await native.wait()
        server.shutdown()
        server.server_close()
        server_thread.join()
