"""Run a selected RustCodeGraph stdio server against a private index snapshot."""

import asyncio
from contextlib import closing
import json
from pathlib import Path
import sqlite3
import tempfile
from uuid import uuid4


SANDBOX = """
(version 1)
(allow default)
; Keep system credential services outside the managed child boundary.
(deny mach-lookup mach-register appleevent-send)
(deny ipc-posix-shm*)
(deny ipc-posix-sem*)
(deny file-read* (regex #"/(Keychains|keychains)(/|$)"))
(deny file-read-data (subpath "/Users") (subpath "/private/tmp")
    (subpath "/private/var/folders") (subpath "/Volumes"))
(allow file-read-data (literal (param "command")) (subpath (param "workspace"))
    (subpath (param "mirror")))
(deny file-read-data (subpath (param "git")) (subpath (param "index"))
    (literal (param "env-file")) (literal (param "mount-config")))
(deny file-write*)
(allow file-write* (subpath (param "mirror")) (literal "/dev/null"))
(deny process-fork process-exec network*)
(allow process-exec (literal (param "command")))
"""

LIMIT = 4 * 1024 * 1024


def frame(message):
    return json.dumps({'jsonrpc': '2.0', **message}).encode() + b'\n'


class Graph:
    def __init__(self, definition, workspace, state):
        args = definition.get('args', [])
        command = Path(definition['command'])
        if (
            definition.get('type') not in (None, 'stdio') or definition.get('env')
            or not command.is_absolute() or command.name != 'rustcodegraph'
            or len(args) != 5 or args[:3] != ['serve', '--mcp', '--path']
            or Path(args[3]).resolve(strict=True) != workspace or args[4] != '--no-watch'
        ):
            raise ValueError('Corust MCP requires a selected RustCodeGraph stdio server')
        self.command = command.resolve(strict=True)
        if not self.command.is_file():
            raise ValueError('Missing RustCodeGraph executable')
        self.workspace = workspace
        self.state = state
        self.private = None
        self.process = None

    def snapshot(self):
        index = self.workspace / '.rustcodegraph'
        source = index / 'rustcodegraph.db'
        if index.is_symlink() or source.is_symlink() or not source.is_file():
            raise ValueError('Corust requires an indexed workspace')
        self.private = tempfile.TemporaryDirectory(prefix='corust-mcp-', dir=self.state)
        mirror = Path(self.private.name)
        (mirror / '.rustcodegraph').mkdir()
        with (
            closing(sqlite3.connect(source.as_uri() + '?mode=rw', uri=True, timeout=1)) as original,
            closing(sqlite3.connect(mirror / '.rustcodegraph/rustcodegraph.db')) as snapshot
        ):
            original.execute('PRAGMA query_only=ON')
            original.backup(snapshot)
        for entry in self.workspace.iterdir():
            if not entry.name.startswith('.') and not entry.is_symlink():
                (mirror / entry.name).symlink_to(entry, target_is_directory=entry.is_dir())
        return mirror

    async def request(self, method, params):
        identifier = uuid4().hex
        self.process.stdin.write(frame({'id': identifier, 'method': method, 'params': params}))
        await self.process.stdin.drain()
        while line := await self.process.stdout.readline():
            reply = json.loads(line)
            if reply.get('id') == identifier and 'method' not in reply:
                if not isinstance(reply.get('result'), dict):
                    raise ValueError('RustCodeGraph rejected MCP request')
                return reply['result']
            if 'method' in reply and 'id' in reply:
                raise ValueError('RustCodeGraph sent an unexpected MCP callback')
        raise ValueError('RustCodeGraph disconnected')

    async def catalog(self, allowed):
        mirror = self.snapshot()
        sandbox = ['/usr/bin/sandbox-exec']
        for key, value in {
            'command': self.command, 'workspace': self.workspace, 'mirror': mirror,
            'git': self.workspace / '.git', 'index': self.workspace / '.rustcodegraph',
            'env-file': self.workspace / '.env', 'mount-config': self.workspace / '.vwacprc.json',
        }.items():
            sandbox.extend(['-D', f'{key}={value}'])
        self.process = await asyncio.create_subprocess_exec(
            *sandbox, '-p', SANDBOX, str(self.command), 'serve', '--mcp', '--path',
            str(mirror), '--no-watch', cwd=mirror,
            env={'HOME': str(mirror), 'TMPDIR': str(mirror), 'RUSTCODEGRAPH_NO_DAEMON': '1'},
            stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.DEVNULL, limit=LIMIT,
        )
        hello = await self.request('initialize', {
            'protocolVersion': '2024-11-05', 'capabilities': {},
            'clientInfo': {'name': 'shiju-corust', 'version': '1'},
        })
        if 'tools' not in hello.get('capabilities', {}):
            raise ValueError('RustCodeGraph did not advertise tools')
        self.process.stdin.write(frame({'method': 'notifications/initialized'}))
        await self.process.stdin.drain()
        result = await self.request('tools/list', {})
        if result.get('nextCursor'):
            raise ValueError('Unexpected paginated RustCodeGraph catalog')
        catalog = {tool['name']: tool for tool in result['tools'] if tool.get('name') in allowed}
        node = catalog.get('rustcodegraph_node')
        if node:
            node['description'] += (
                ' Use exact relative paths found in the graph or in source already retrieved. '
                'When only the symbol name is known, omit file and search by name.'
            )
        return catalog

    async def call(self, name, arguments):
        # Always resolve against the private snapshot, regardless of the model's spelling.
        arguments = {key: value for key, value in arguments.items() if key != 'projectPath'}
        if name == 'rustcodegraph_node' and arguments.get('file'):
            requested = Path(arguments['file'])
            resolved = (self.workspace / requested).resolve()
            if not resolved.is_relative_to(self.workspace) or any(
                part.startswith('.') for part in resolved.relative_to(self.workspace).parts
            ):
                raise ValueError('Corust MCP node path is outside the indexed source')
            arguments['file'] = str(resolved.relative_to(self.workspace))
        return await self.request('tools/call', {'name': name, 'arguments': arguments})

    async def close(self):
        child, self.process = self.process, None
        if child:
            if child.returncode is None:
                try:
                    child.kill()
                except ProcessLookupError:
                    pass
            await child.wait()
        if self.private:
            self.private.cleanup()
            self.private = None
