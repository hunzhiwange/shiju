"""Expose selected stdio MCP tools to the native Poolside HTTP client."""

import asyncio
from concurrent.futures import CancelledError
from hmac import compare_digest
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
from pathlib import Path
import re
import runpy
from secrets import token_hex
from threading import Lock, Thread


LIMIT = 4 * 1024 * 1024
TIMEOUT = 15
NAME = re.compile(r'[a-zA-Z0-9]+(?:_[a-zA-Z0-9]+)*')
POLICY = 'shiju.poolside.toolPolicy'


def arguments_key(arguments):
    return json.dumps(arguments, sort_keys=True, separators=(',', ':'), allow_nan=False)


class Handler(BaseHTTPRequestHandler):
    def setup(self):
        self.request.settimeout(5)
        super().setup()

    def log_message(self, *_):
        pass

    def respond(self, status, body=b''):
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        if body:
            self.wfile.write(body)

    def do_GET(self):
        self.respond(405)

    do_DELETE = do_GET

    def do_POST(self):
        mounts = self.server.mounts
        server = self.path.removeprefix('/')
        token = mounts.tokens.get(server)
        if token is None or not compare_digest(
            self.headers.get('Authorization', '').encode(), ('Bearer ' + token).encode()
        ):
            self.respond(403)
            return
        try:
            length = int(self.headers.get('Content-Length', '0'))
            if not 0 < length <= LIMIT:
                self.respond(400)
                return
            request = json.loads(self.rfile.read(length))
            if not isinstance(request, dict) or request.get('jsonrpc') != '2.0':
                self.respond(400)
                return
            if 'id' not in request:
                self.respond(202)
                return
            reply = {'jsonrpc': '2.0', 'id': request['id']}
            try:
                reply['result'] = mounts.request(server, request['method'], request.get('params', {}))
            except (KeyError, ValueError, TypeError, RuntimeError, TimeoutError, CancelledError, OSError):
                reply['error'] = {'code': -32000, 'message': 'Poolside MCP request rejected or failed'}
            payload = json.dumps(reply, allow_nan=False).encode()
            self.respond(200 if len(payload) <= LIMIT else 500, payload if len(payload) <= LIMIT else b'')
        except (ValueError, OSError):
            self.close_connection = True


class Mounts:
    def __init__(self, workspace, settings):
        self.workspace = workspace
        self.settings = settings
        self.lock = Lock()
        self.catalogs = {}
        self.peers = {}
        self.tokens = {}
        self.turn = None
        self.calls = {}
        self.pending = {}
        self.approved = []
        self.worker = None
        self.http = None
        self.stdio = None

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.revoke()
        if self.http is not None:
            self.http.shutdown()
            self.http_thread.join()
            self.http.server_close()
            asyncio.run_coroutine_threadsafe(self.drain(), self.loop).result()
            self.loop.call_soon_threadsafe(self.loop.stop)
            self.loop_thread.join()
            self.loop.close()

    async def drain(self):
        current = asyncio.current_task()
        tasks = [task for task in asyncio.all_tasks() if task is not current]
        await asyncio.gather(*tasks, return_exceptions=True)

    async def discover(self, definitions):
        self.stdio = runpy.run_path(str(Path(__file__).with_name('shiju-stdio.py')))
        for definition in definitions:
            name = definition.get('name', '')
            if not NAME.fullmatch(name) or name in self.catalogs:
                raise ValueError('Invalid or duplicate Poolside MCP service name')
            policy = (definition.get('_meta') or {}).get(POLICY)
            if not isinstance(policy, dict):
                raise ValueError('Poolside MCP requires the resolved tool policy')
            peer = self.stdio['Stdio'](definition, self.workspace)
            tools = {}
            observed = set()
            policy_definition = {'_meta': {'shiju.cortex.toolPolicy': policy}}
            try:
                async for tool in peer.list_tools():
                    tool_name = tool['name']
                    if (not NAME.fullmatch(tool_name) or len(name + '__' + tool_name) > 64
                            or not isinstance(tool.get('inputSchema'), dict)
                            or len(observed) >= 256 or tool_name in observed):
                        raise ValueError('Invalid Poolside MCP tool catalog')
                    observed.add(tool_name)
                    if self.stdio['allowed_tool'](policy_definition, name, tool_name):
                        tools[tool_name] = {key: tool[key] for key in ('name', 'description', 'inputSchema') if key in tool}
            finally:
                await peer.close()
            if not observed:
                raise ValueError('Selected Poolside MCP server has no callable tools')
            self.peers[name] = definition
            self.catalogs[name] = tools

    def install(self, definitions):
        if self.http is not None or self.catalogs:
            raise ValueError('Poolside MCP settings are bound to a single new session')
        # Discover before publishing a native endpoint or updating its private config.
        try:
            asyncio.run(asyncio.wait_for(self.discover(definitions), TIMEOUT))
        except Exception:
            self.catalogs.clear()
            self.peers.clear()
            raise
        self.http = HTTPServer(('127.0.0.1', 0), Handler)
        self.http.mounts = self
        self.loop = asyncio.new_event_loop()
        self.loop_thread = Thread(target=self.loop.run_forever)
        self.http_thread = Thread(target=self.http.serve_forever)
        self.loop_thread.start()
        self.http_thread.start()
        config = json.loads(self.settings.read_text())
        entries = {}
        for name in self.catalogs:
            self.tokens[name] = token_hex(32)
            entries[name] = {'transport': {
                'url': f'http://127.0.0.1:{self.http.server_port}/{name}',
                'type': 'http',
                'headers': ['Authorization: Bearer ' + self.tokens[name]],
            }}
        config['mcp_servers'] = entries
        replacement = self.settings.with_suffix('.next')
        replacement.write_text(json.dumps(config))
        replacement.replace(self.settings)

    def begin(self, session):
        with self.lock:
            self.turn = session

    def end(self):
        with self.lock:
            self.turn = None
            self.calls.clear()
            self.pending.clear()
            self.approved.clear()
            if self.worker:
                self.worker.cancel()

    def revoke(self):
        self.end()
        self.tokens.clear()

    def observe(self, session, update):
        if not self.catalogs or update.get('sessionUpdate') != 'tool_call':
            return
        name, _, tool = update.get('title', '').partition('__')
        arguments = update.get('rawInput')
        if tool in self.catalogs.get(name, {}) and isinstance(arguments, dict):
            with self.lock:
                if self.turn == session:
                    self.calls[update['toolCallId']] = (name, tool, arguments_key(arguments), arguments)

    def permission(self, identifier, params):
        tool = params.get('toolCall', {})
        options = [option for option in params.get('options', [])
                   if option.get('kind') == 'allow_once']
        with self.lock:
            call = self.calls.pop(tool.get('toolCallId'), None)
            if (self.turn is None or self.turn != params.get('sessionId') or call is None
                    or tool.get('title') != ':'.join(('mcp', *call[:2])) or len(options) != 1):
                return False
            params['options'] = options + [option for option in params['options']
                                           if option.get('kind') == 'reject_once']
            tool['rawInput'] = call[3]
            self.pending[identifier] = (options[0]['optionId'], call[:3])
            return True

    def decide(self, identifier, result):
        with self.lock:
            entry = self.pending.pop(identifier, None)
            if entry and isinstance(result, dict) and result.get('outcome') == {
                'outcome': 'selected', 'optionId': entry[0]
            }:
                self.approved.append(entry[1])

    def claim(self, server, tool, arguments):
        # Called under the turn lock by the HTTP dispatcher.
        call = (server, tool, arguments_key(arguments))
        if self.turn is not None and call in self.approved:
            self.approved.remove(call)
            return call

    async def execute(self, server, tool, arguments):
        peer = self.stdio['Stdio'](self.peers[server], self.workspace)
        try:
            return await asyncio.wait_for(peer.call(tool, arguments), TIMEOUT)
        finally:
            await peer.close()

    def request(self, server, method, params):
        if method == 'initialize':
            return {'protocolVersion': '2024-11-05', 'capabilities': {'tools': {}},
                    'serverInfo': {'name': server, 'version': '1'}}
        if method == 'tools/list':
            return {'tools': list(self.catalogs[server].values())}
        if method != 'tools/call' or not isinstance(params.get('arguments'), dict):
            raise ValueError('Unsupported Poolside MCP request')
        with self.lock:
            if self.claim(server, params['name'], params['arguments']) is None:
                raise ValueError('No matching approval')
            worker = asyncio.run_coroutine_threadsafe(
                self.execute(server, params['name'], params['arguments']), self.loop)
            self.worker = worker
        try:
            return worker.result(TIMEOUT + 2)
        finally:
            with self.lock:
                worker.cancel()
                self.worker = None
