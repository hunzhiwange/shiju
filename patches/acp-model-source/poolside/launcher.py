#!/usr/bin/env python3
"""Run the reviewed Poolside engine and publish staged text through the ACP host."""

import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
from tempfile import TemporaryDirectory
from urllib.parse import urlsplit


class StagedWrites:
    def __init__(self, root):
        self.root = root
        self.pending = {}
        self.completed = {}

    def observe(self, update):
        id_ = update.get('toolCallId')
        if update.get('sessionUpdate') == 'tool_call':
            changes = {}
            for entry in update.get('content', []):
                if entry.get('type') != 'diff' or not isinstance(entry.get('newText'), str):
                    continue
                path = Path(entry['path'])
                try:
                    if (path.is_absolute() and path != self.root
                            and '.git' not in path.relative_to(self.root).parts
                            and path.resolve() == path):
                        changes[str(path)] = entry['newText']
                except (ValueError, OSError, RuntimeError):
                    continue
            self.pending[id_] = changes
        if update.get('status') == 'completed':
            self.completed.update(self.pending.pop(id_, {}))
        elif update.get('status') == 'failed':
            self.pending.pop(id_, None)

    def take(self):
        completed, self.completed = self.completed, {}
        self.pending.clear()
        return completed


def send(stream, frame):
    stream.write(json.dumps(frame, separators=(',', ':')).encode() + b'\n')
    stream.flush()


def result(id_, value):
    return dict(jsonrpc='2.0', id=id_, result=value)


def failed(id_, message):
    return dict(jsonrpc='2.0', id=id_, error=dict(code=-32000, message=message))


def frames(process):
    buffers = {'host': bytearray(), 'pool': bytearray()}
    with selectors.DefaultSelector() as selector:
        selector.register(sys.stdin.buffer, selectors.EVENT_READ, 'host')
        selector.register(process.stdout, selectors.EVENT_READ, 'pool')
        while True:
            for key, _ in selector.select():
                data = os.read(key.fd, 65536)
                if not data:
                    return
                buffer = buffers[key.data]
                buffer.extend(data)
                lines = buffer.split(b'\n')
                buffers[key.data] = lines.pop()
                for line in lines:
                    yield key.data, json.loads(line)


def bridge(process, workspace, mounts):
    staged = StagedWrites(workspace)
    active_prompt = None
    review = None
    host_write = None
    counter = 0
    rejected = False
    mounted_request = None
    mounted_session = None

    def advance_review():
        nonlocal counter, host_write, review
        id_, session, remaining = review
        if remaining and not rejected:
            path, content = remaining.pop(0)
            counter += 1
            host_write = f'shiju-poolside-write-{counter}'
            send(sys.stdout.buffer, dict(jsonrpc='2.0', id=host_write, method='fs/write_text_file',
                                        params=dict(sessionId=session, path=path, content=content)))
        else:
            # Direct application bypasses the host filesystem policy. The host
            # owns all writes. Discard the engine's staged copy either way.
            send(process.stdin, result(id_, dict(outcome=dict(outcome='selected', optionId='sandbox-discard'))))
            host_write = None
            review = None

    for source, frame in frames(process):
        id_, method = frame.get('id'), frame.get('method')
        params = frame.get('params', {})
        if source == 'host':
            if method is None and host_write is not None and id_ == host_write:
                rejected |= 'result' not in frame
                advance_review()
                continue
            if method in ('session/new', 'session/load', 'session/resume'):
                if Path(params.get('cwd', '')).resolve() != workspace or mounts.http is not None:
                    send(sys.stdout.buffer, failed(id_, 'Poolside requires the reviewed workspace and a new MCP process'))
                    continue
                if params.get('mcpServers'):
                    if method != 'session/new' or active_prompt is not None:
                        send(sys.stdout.buffer, failed(id_, 'Poolside MCP requires an idle new session'))
                        continue
                    try:
                        mounts.install(params['mcpServers'])
                    except Exception:
                        send(sys.stdout.buffer, failed(id_, 'Poolside could not prepare the selected MCP services'))
                        continue
                    mounted_request = id_
                    params['mcpServers'] = []
                # Use one canonical path for Docker, tool diffs and host policy.
                params['cwd'] = str(workspace)
            if method is None:
                mounts.decide(id_, frame.get('result', {}))
            if mounts.http is not None and (
                (method or '').startswith('_poolside/mcp/')
                or method == 'session/fork'
                or (method == 'session/prompt'
                    and (mounted_session is None or params.get('sessionId') != mounted_session))
            ):
                send(sys.stdout.buffer, failed(id_, 'Poolside MCP is bound to the selected session'))
                continue
            if method == 'session/prompt':
                if active_prompt is not None:
                    send(sys.stdout.buffer, failed(id_, 'Poolside already has an active prompt'))
                    continue
                active_prompt = (id_, params.get('sessionId'))
                if mounted_session is not None:
                    mounts.begin(mounted_session)
                rejected = False
                staged.take()
            if method == 'session/cancel' and active_prompt and params.get('sessionId') == active_prompt[1]:
                rejected = True
                mounts.end()
            if (method in ('session/close', 'session/delete') and mounted_session is not None
                    and mounted_session == params.get('sessionId')):
                mounts.revoke()
                mounted_session = None
            send(process.stdin, frame)
            continue

        if method == 'session/update' and active_prompt and params.get('sessionId') == active_prompt[1]:
            staged.observe(params['update'])
            mounts.observe(params['sessionId'], params['update'])
        if method is None and id_ is not None and id_ == mounted_request:
            mounted_request = None
            mounted_session = frame.get('result', {}).get('sessionId')
            if mounted_session is None:
                mounts.revoke()
        if method == 'session/request_permission' and params.get('toolCall', {}).get('title', '').startswith('mcp:'):
            if not mounts.permission(id_, params):
                send(process.stdin, result(id_, dict(outcome=dict(outcome='cancelled'))))
                continue
        if method == 'session/request_permission' and params.get('toolCall', {}).get('toolCallId') == 'poolside/sandbox_review':
            session = params.get('sessionId')
            options = {option.get('optionId') for option in params.get('options', [])}
            if review is not None or not active_prompt or session != active_prompt[1] or 'sandbox-discard' not in options:
                send(process.stdin, result(id_, dict(outcome=dict(outcome='cancelled'))))
                rejected = True
                continue
            review = (id_, session, list(staged.take().items()))
            advance_review()
            continue
        if method == 'fs/write_text_file':
            send(process.stdin, failed(id_, 'Poolside writes must use the reviewed sandbox staging flow'))
            rejected = True
            continue
        if method is None and active_prompt and id_ == active_prompt[0]:
            mounts.end()
            if rejected or staged.take() or review is not None:
                frame = failed(id_, 'Poolside staged writes were not accepted by the host')
            active_prompt = None
        send(sys.stdout.buffer, frame)


def managed_environment(directory, workspace):
    home = directory / 'home'
    config = home / '.config' / 'poolside'
    config.mkdir(parents=True)
    # The engine may use the local Docker socket to create a read-only sandbox.
    # The sandbox does not mount that socket or receive the native process env.
    docker = Path.home() / '.docker/run/docker.sock'
    if not docker.is_socket():
        docker = Path('/var/run/docker.sock')
    if not docker.is_socket():
        raise ValueError('Poolside requires a running local Docker engine')
    tools = {'todo_action', 'question', 'shell', 'shell_list', 'shell_tail', 'shell_status',
             'shell_kill', 'shell_send', 'shell_wait', 'skill', 'subagent', 'switch_mode',
             'list_secrets', 'web_fetch', 'edit'}
    settings = {
        'tools': {name: {'disabled': True} for name in sorted(tools)},
        'paths': {
            'allow': [{'path': workspace.as_posix() + '/**', 'write': True}],
            'deny': [{'path': workspace.as_posix() + '/.git/**'}],
        },
        'sandbox': {
            'image': 'node@sha256:18a6d90fd60542269850c80ca03b29b1d9b14d6b221ac7ee8e35b0d33ef904bd',
            'filesystem': {'workspaces': {'access': 'read-only'}},
        },
    }
    with (config / 'settings.yaml').open('x') as stream:
        json.dump(settings, stream)
    env = {
        'HOME': str(home), 'XDG_CONFIG_HOME': str(home / '.config'),
        'XDG_DATA_HOME': str(home / '.local/share'), 'XDG_CACHE_HOME': str(home / '.cache'),
        'TMPDIR': str(directory), 'PATH': '/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin',
        'DOCKER_HOST': f'unix://{docker}',
    }
    if 'VW_ACP_LAUNCH_REFERENCE' in os.environ:
        endpoint = os.environ.get('SHIJU_POOLSIDE_RELAY_URL', '')
        model = os.environ.get('SHIJU_POOLSIDE_RELAY_MODEL', '')
        token = os.environ.get('SHIJU_POOLSIDE_RELAY_TOKEN', '')
        url = urlsplit(endpoint)
        if (url.scheme != 'http' or url.hostname != '127.0.0.1' or url.port is None
                or url.path or url.query or url.fragment or url.username is not None
                or not model or not token):
            raise ValueError('Poolside requires a private loopback model binding')
        env.update(POOLSIDE_STANDALONE_BASE_URL=endpoint,
                   POOLSIDE_STANDALONE_MODEL=model, POOLSIDE_API_KEY=token)
    return env


def main():
    executable = Path(__file__).resolve().with_name('pool-native')
    if sys.argv[1:] == ['--version']:
        os.execve(executable, [str(executable), '--version'], {})
    if sys.argv[1:] != ['acp']:
        raise ValueError('Poolside requires reviewed ACP arguments')
    sys.dont_write_bytecode = True
    from mcp_bridge import Mounts

    os.umask(0o077)
    workspace = Path.cwd().resolve()
    for parent in (workspace, *workspace.parents):
        for name in ('.poolside', '.env'):
            if os.path.lexists(parent / name):
                raise ValueError('Poolside project configuration overrides are not verified')
    with TemporaryDirectory(prefix='shiju-poolside-') as temp:
        env = managed_environment(Path(temp), workspace)
        process = subprocess.Popen([str(executable), 'acp', '--sandbox', 'required'],
                                   env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        def stop(signum, _frame):
            raise SystemExit(128 + signum)
        for sig in (signal.SIGTERM, signal.SIGINT):
            signal.signal(sig, stop)
        with Mounts(workspace, Path(env['XDG_CONFIG_HOME']) / 'poolside/settings.yaml') as mounts:
            try:
                bridge(process, workspace, mounts)
            finally:
                mounts.end()
                process.stdin.close()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.terminate()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
        return process.returncode


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (OSError, ValueError) as error:
        print(error, file=sys.stderr)
        sys.exit(1)
