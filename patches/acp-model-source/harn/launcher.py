#!/usr/bin/env python3
"""Launch the pinned Harn engine; grant its relay credential to in-process LLM calls only."""
import asyncio
import json
import hashlib
import os
from pathlib import Path
import runpy
import signal
import sys

# Load the digest-verified source directly, without consulting Python bytecode caches.
_stdio = runpy.run_path(str(Path(__file__).resolve().with_name('shiju_stdio.py')))
APPROVAL_PREFIX, Bridge, LIMIT, NAME, TIMEOUT, fail, lines = (
    _stdio[key] for key in ('APPROVAL_PREFIX', 'Bridge', 'LIMIT', 'NAME', 'TIMEOUT', 'fail', 'lines')
)


def grant_request(frame, has_binding):
    if has_binding and frame.get('method') == 'session/new':
        params = frame.setdefault('params', {})
        if 'environmentPolicy' in params:
            raise ValueError('Harn managed sessions cannot override the relay environment policy')
        params['environmentPolicy'] = {'kind': 'granted', 'grants': [{
            'name': 'session_model_relay',
            'source': {'secret_env': {'var': 'SHIJU_HARN_RELAY_TOKEN'}},
            'expose_as_env': 'SHIJU_HARN_RELAY_TOKEN', 'expose_to': 'in_process',
        }]}
    return frame


class HarnBridge(Bridge):
    def __init__(self, native, workspace, has_binding):
        super().__init__(native, workspace)
        self.has_binding = has_binding
        self.file_calls = {}
        self.session_directories = {}
        self.admitted_calls = set()

    async def answer(self, identifier, value):
        await self.send({'id': identifier, 'result': value})

    async def from_host(self, reader):
        async for line in lines(reader):
            frame = json.loads(line)
            method, identifier = frame.get('method'), frame.get('id')
            params = frame.get('params', {})
            sid = params.get('sessionId')
            if not method and isinstance(identifier, str) and identifier.startswith(APPROVAL_PREFIX):
                waiter = self.pending.pop(identifier, None)
                if waiter and not waiter.done():
                    waiter.set_result(frame.get('result', {}))
                continue
            if method in ('session/new', 'session/load', 'session/resume'):
                if self.turn or self.setup:
                    self.host(fail(identifier, 'Harn session is busy'))
                    continue
                try:
                    cwd = Path(params['cwd']).resolve(strict=True)
                    if cwd != self.workspace:
                        raise ValueError('Harn workspace mismatch')
                    definitions = params.get('mcpServers', [])
                    if (not isinstance(definitions, list) or len(definitions) > 16
                            or any(not isinstance(d, dict) or not NAME.fullmatch(d.get('name', ''))
                                   for d in definitions)):
                        raise ValueError('Invalid Harn MCP service selection')
                    # The existing sandbox adapter enforces the resolved host
                    # tool policy. Supply only this backend's trusted metadata.
                    definitions = [{**d, '_meta': {'shiju.cortex.toolPolicy':
                                     (d.get('_meta') or {}).get('shiju.harn.toolPolicy', {})}}
                                   for d in definitions]
                    grant_request(frame, self.has_binding)
                    selection = await asyncio.wait_for(self.selection(definitions), TIMEOUT)
                    params['mcpServers'] = []
                except (OSError, KeyError, ValueError, AttributeError, TypeError, RuntimeError, TimeoutError):
                    self.host(fail(identifier, 'Could not configure explicit Harn MCP tools or session environment'))
                    continue
                self.setup = (identifier, sid, selection, params['cwd'])
            elif method == 'session/fork':
                if self.turn or self.setup or self.sessions.get(sid) or params.get('mcpServers'):
                    self.host(fail(identifier, 'Harn MCP sessions cannot be forked'))
                    continue
                self.setup = (identifier, None, {}, self.session_directories.get(sid, str(self.workspace)))
            elif method == 'session/prompt':
                if self.turn or self.setup or sid not in self.sessions:
                    self.host(fail(identifier, 'Harn prompt has no idle session'))
                    continue
                self.turn = (identifier, sid)
                self.cancelled = False
                self.admitted_calls.clear()
            if self.turn and (method in ('session/cancel', 'session/close', 'session/stop')
                              and sid == self.turn[1]
                              or method == '$/cancel_request' and params.get('requestId') == self.turn[0]):
                self.cancelled = True
                await self.send(frame)
                await self.cancel_calls()
                await self.stop(self.sessions.get(self.turn[1]) if self.turn else None)
                if method in ('session/close', 'session/stop'):
                    self.sessions.pop(sid, None)
                    self.session_directories.pop(sid, None)
                continue
            if method in ('session/close', 'session/stop'):
                await self.stop(self.sessions.pop(sid, None))
                self.session_directories.pop(sid, None)
            if not method and self.file_calls.pop(identifier, None) == 'read_file' and 'result' in frame:
                frame['result'] = frame['result']['content']
            await self.send(frame)

    async def from_native(self):
        async for line in lines(self.native.stdout):
            frame = json.loads(line)
            method, identifier = frame.get('method'), frame.get('id')
            params = frame.get('params', {})
            sid = params.get('sessionId')
            if self.setup and not method and identifier == self.setup[0]:
                _, restored, selected, directory = self.setup
                self.setup = None
                if 'result' in frame:
                    session = frame['result'].get('sessionId', restored)
                    if session:
                        await self.stop(self.sessions.get(session))
                        self.sessions[session] = selected
                        self.session_directories[session] = directory
                else:
                    await self.stop(selected)
            if self.turn and not method and identifier == self.turn[0]:
                await self.cancel_calls()
                await self.stop(self.sessions.get(self.turn[1]))
                self.turn = None
            if method == 'session/update' and self.turn and sid == self.turn[1]:
                update = params['update']
                title, call_id = update.get('title'), update.get('toolCallId')
                arguments = update.get('rawInput')
                # Streamed calls are announced with an empty argument object.
                # The execution update carries the arguments used by the handler.
                if (not self.cancelled and update.get('sessionUpdate') == 'tool_call_update'
                        and update.get('status') == 'in_progress'
                        and title in self.sessions.get(sid, {})
                        and isinstance(call_id, str) and isinstance(arguments, dict)
                        and call_id not in self.admitted_calls):
                    self.admitted_calls.add(call_id)
                    self.calls[call_id] = (title, arguments)
            if method == 'builtin_call':
                await self.handle_builtin(identifier, params)
            else:
                self.host(frame)

    async def handle_builtin(self, identifier, params):
        name, sid = params.get('name'), params.get('sessionId')
        args = params.get('args', [])
        if (not self.turn or sid != self.turn[1] or self.cancelled
                or len(args) != 1 or not isinstance(args[0], dict)):
            await self.send(fail(identifier, 'Harn host call has no active prompt'))
            return
        values = args[0]
        if name == 'shiju_mcp_catalog':
            await self.answer(identifier, [entry[2] for entry in self.sessions[sid].values()])
            return
        if name == 'shiju_mcp_call':
            name = values.get('name')
            if not isinstance(name, str):
                await self.send(fail(identifier, 'Invalid Harn MCP tool'))
                return
            task = asyncio.create_task(self.call(identifier, name, values.get('arguments')))
            self.workers.add(task)
            task.add_done_callback(self.workers.discard)
            return
        path = values.get('path')
        if (name not in ('read_file', 'write_file') or not isinstance(path, str) or not path
                or chr(0) in path or (name == 'write_file' and not isinstance(values.get('content'), str))):
            await self.send(fail(identifier, 'Unverified Harn host tool'))
            return
        translated = {'sessionId': sid, 'path': os.path.normpath(os.path.join(self.session_directories[sid], path))}
        if name == 'write_file':
            translated['content'] = values['content']
        self.file_calls[identifier] = name
        self.host({'id': identifier, 'params': translated,
                   'method': 'fs/read_text_file' if name == 'read_file' else 'fs/write_text_file'})


async def run(command, env):
    loop = asyncio.get_running_loop()
    reader = asyncio.StreamReader(limit=LIMIT)
    await loop.connect_read_pipe(lambda: asyncio.StreamReaderProtocol(reader), sys.stdin.buffer)
    native = await asyncio.create_subprocess_exec(
        *command, env=env, stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE, limit=LIMIT,
    )
    bridge = HarnBridge(native, Path.cwd().resolve(), bool(env.get('SHIJU_HARN_RELAY_TOKEN')))
    stop = asyncio.Event()
    for sig in (signal.SIGINT, signal.SIGTERM):
        loop.add_signal_handler(sig, stop.set)
    peer = asyncio.create_task(bridge.from_native())
    tasks = {peer, asyncio.create_task(bridge.from_host(reader)), asyncio.create_task(stop.wait())}
    try:
        done, _ = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
        for task in done:
            task.result()
        return await native.wait() if peer in done else 0
    finally:
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)
        await bridge.cancel_calls()
        for selection in bridge.sessions.values():
            await bridge.stop(selection)
        if native.returncode is None:
            native.terminate()
            try:
                await asyncio.wait_for(native.wait(), 3)
            except TimeoutError:
                native.kill()
                await native.wait()


def main():
    if len(sys.argv) != 1:
        raise ValueError('Harn managed launcher does not accept custom arguments')
    root = Path(__file__).resolve().parent
    # Never hand unrelated gateway credentials or user runtime hooks to the engine.
    allowed = {'PATH', 'LANG', 'LC_ALL', 'TMPDIR', 'SYSTEMROOT', 'NO_PROXY', 'no_proxy',
               'HARN_PROVIDERS_CONFIG', 'HARN_SESSION_STORE_ROOT', 'HARN_STATE_DIR',
               'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME', 'SHIJU_HARN_RELAY_TOKEN'}
    env = {key: value for key, value in os.environ.items() if key in allowed}
    pipeline = Path(os.environ.get('SHIJU_HARN_PIPELINE', str(root / 'agent.harn')))
    if hashlib.sha256(pipeline.read_bytes()).digest() != hashlib.sha256((root / 'agent.harn').read_bytes()).digest():
        raise ValueError('Harn private pipeline differs from the reviewed artifact')
    return asyncio.run(run([str(root / 'harn-native'), 'serve', 'acp',
                            '--session-environment', 'isolated', str(pipeline)], env))


if __name__ == '__main__':
    sys.exit(main())
