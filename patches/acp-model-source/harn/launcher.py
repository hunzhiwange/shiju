#!/usr/bin/env python3
"""Launch the pinned Harn engine; grant its relay credential to in-process LLM calls only."""
import json
import hashlib
import os
from pathlib import Path
import signal
import subprocess
import sys
import threading


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
    process = subprocess.Popen([str(root / 'harn-native'), 'serve', 'acp',
                                '--session-environment', 'isolated', str(pipeline)],
                               env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    input_lock = threading.Lock()
    output_lock = threading.Lock()
    pending_sessions = {}
    sessions = {}
    host_calls = {}
    def to_engine(frame):
        with input_lock:
            process.stdin.write(json.dumps(frame, separators=(',', ':')).encode() + b'\n')
            process.stdin.flush()
    def to_host(frame):
        with output_lock:
            sys.stdout.buffer.write(json.dumps(frame, separators=(',', ':')).encode() + b'\n')
            sys.stdout.buffer.flush()
    def stop(signum, _frame):
        if process.poll() is None:
            process.send_signal(signum)
    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    def forward():
        try:
            for line in sys.stdin.buffer:
                frame = grant_request(json.loads(line), bool(env.get('SHIJU_HARN_RELAY_TOKEN')))
                if frame.get('method') in ('session/new', 'session/load', 'session/resume'):
                    pending_sessions[frame['id']] = frame['params']['cwd']
                if 'method' not in frame and frame.get('id') in host_calls:
                    kind = host_calls.pop(frame['id'])
                    if 'result' in frame and kind == 'read_file':
                        frame['result'] = frame['result']['content']
                to_engine(frame)
        except (ValueError, KeyError, BrokenPipeError):
            process.terminate()
        finally:
            with input_lock:
                process.stdin.close()
    def receive():
        try:
            for line in process.stdout:
                frame = json.loads(line)
                if 'method' not in frame and frame.get('id') in pending_sessions:
                    cwd = pending_sessions.pop(frame['id'])
                    if 'result' in frame and 'sessionId' in frame['result']:
                        sessions[frame['result']['sessionId']] = cwd
                if frame.get('method') == 'builtin_call':
                    params = frame['params']
                    name = params.get('name')
                    cwd = sessions.get(params.get('sessionId'))
                    args = params.get('args', [])
                    if name not in ('read_file', 'write_file') or cwd is None or len(args) != 1:
                        to_engine({'jsonrpc':'2.0', 'id':frame['id'], 'error':{
                            'code':-32601, 'message':'Unverified Harn host tool'}})
                        continue
                    values = args[0]
                    path = values['path']
                    if not isinstance(path, str) or not path or '\0' in path:
                        raise ValueError('Invalid Harn file path')
                    absolute = os.path.normpath(os.path.join(cwd, path))
                    translated = {'sessionId':params['sessionId'], 'path':absolute}
                    if name == 'write_file':
                        translated['content'] = values['content']
                    host_calls[frame['id']] = name
                    frame = {'jsonrpc':'2.0', 'id':frame['id'],
                             'method':'fs/read_text_file' if name == 'read_file' else 'fs/write_text_file',
                             'params':translated}
                to_host(frame)
        except (ValueError, KeyError, BrokenPipeError):
            process.terminate()
    threading.Thread(target=forward, daemon=True).start()
    receiver = threading.Thread(target=receive, daemon=True)
    receiver.start()
    try:
        result = process.wait()
        receiver.join(timeout=5)
        return result
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait()


if __name__ == '__main__':
    sys.exit(main())
