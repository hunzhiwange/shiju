"""Probe submitted programs with real Git pushes, authenticated clones and TLS."""
import base64
from contextlib import contextmanager
import json
import os
from pathlib import Path
import socket
import ssl
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request


def run(*args, cwd=None, ok=True):
    result = subprocess.run(args, cwd=cwd, env={**os.environ, 'GIT_CONFIG_NOSYSTEM': '1',
                            'GIT_CONFIG_GLOBAL': '/dev/null', 'GIT_TERMINAL_PROMPT': '0'},
                            capture_output=True, timeout=15)
    if ok:
        assert result.returncode == 0, result.stderr.decode(errors='replace')
    return result


def git(path, *args):
    return run('git', '-C', str(path), *args).stdout


def revision(client, branch, content):
    git(client, 'checkout', '-B', branch)
    (client / 'index.html').write_bytes(content)
    (client / 'nested').mkdir(exist_ok=True)
    (client / 'nested/data.bin').write_bytes(content + b'\x00\xff')
    git(client, 'add', '.')
    git(client, '-c', 'user.name=Synthetic', '-c', 'user.email=fixture@example.invalid',
        'commit', '-m', branch)


def push(client, *branches):
    start = time.monotonic()
    git(client, 'push', 'origin', *branches)
    assert time.monotonic() - start <= 3, 'deployment exceeded 3 seconds'


def check_deploy(candidate, root, child):
    script = candidate / 'setup.sh'
    assert script.is_file(), 'setup.sh missing'
    run('sh', str(script), str(root))
    server = root / 'server.git'
    assert git(server, 'rev-parse', '--is-bare-repository').strip() == b'true'
    assert git(server, 'symbolic-ref', 'HEAD').strip() == b'refs/heads/main'
    assert not git(server, 'for-each-ref').strip(), 'repository not empty'
    client = root / 'client'
    run('git', 'clone', str(server), str(client))
    main, dev = b'main synthetic\n\xc3\xa9', b'development synthetic\n'
    revision(client, 'main', main)
    push(client, 'main')
    revision(client, 'dev', dev)
    push(client, 'dev')
    def verify(a, b):
        for branch, content in [('main', a), ('dev', b)]:
            assert (root / branch / 'index.html').read_bytes() == content, branch
            assert (root / branch / 'nested/data.bin').read_bytes() == content + b'\x00\xff'
    verify(main, dev)
    if child == 'm2':
        return
    revision(client, 'preview', b'never publish this')
    push(client, 'preview')
    verify(main, dev)
    revision(client, 'main', b'main revision two')
    push(client, 'main')
    verify(b'main revision two', dev)
    revision(client, 'dev', b'dev revision two')
    push(client, 'dev')
    verify(b'main revision two', b'dev revision two')
    revision(client, 'main', b'main batch')
    revision(client, 'dev', b'dev batch')
    push(client, 'main', 'dev')
    verify(b'main batch', b'dev batch')


@contextmanager
def service(script, args):
    assert script.is_file(), 'serve.py missing'
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    process = subprocess.Popen([sys.executable, str(script), args[0], str(port), *args[1:]],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(100):
            assert process.poll() is None, 'service exited'
            try:
                with socket.create_connection(('127.0.0.1', port), timeout=.1):
                    break
            except OSError:
                time.sleep(.03)
        else:
            raise AssertionError('service did not listen')
        yield port
    finally:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def response(url, auth=None):
    headers = {} if auth is None else {'Authorization': 'Basic ' + base64.b64encode(auth).decode()}
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers=headers),
                                    context=ssl._create_unverified_context(), timeout=2) as result:
            return result.status, result.read()
    except urllib.error.HTTPError as error:
        return error.code, error.read()


def check_auth(candidate, root):
    server, client = root / 'server.git', root / 'writer'
    run('git', 'init', '--bare', '--initial-branch=main', str(server))
    run('git', 'clone', str(server), str(client))
    payload = b'authenticated synthetic repository\x00\xff'
    revision(client, 'main', payload)
    push(client, 'main')
    git(server, 'update-server-info')
    with service(candidate / 'serve.py', [str(root)]) as port:
        url = f'http://127.0.0.1:{port}/server.git'
        for auth in (None, b'sandbox:wrong', b'other:synthetic-pass'):
            assert response(url + '/HEAD', auth)[0] == 401, 'invalid credentials accepted'
        assert response(url + '/HEAD', b'sandbox:synthetic-pass') == (200, b'ref: refs/heads/main\n')
        header = 'Authorization: Basic ' + base64.b64encode(b'sandbox:synthetic-pass').decode()
        clone = root / 'reader'
        run('git', '-c', 'http.extraHeader='+header, 'clone', url, str(clone))
        assert (clone / 'index.html').read_bytes() == payload
        assert git(clone, 'rev-parse', 'HEAD') == git(client, 'rev-parse', 'HEAD')
        assert run('git', '-c', 'http.extraHeader=Authorization: Basic YmFkOmJhZA==',
                   'clone', url, str(root / 'denied'), ok=False).returncode != 0


def check_https(candidate, root):
    for branch in ('main', 'dev'):
        (root / branch / 'nested').mkdir(parents=True)
        (root / branch / 'index.html').write_bytes(branch.encode() + b'\x00\xff\n')
        (root / branch / 'nested/data.bin').write_bytes(branch.encode() + b' nested\x00')
    cert, key = root / 'cert.pem', root / 'key.pem'
    run('openssl', 'req', '-x509', '-nodes', '-days', '1', '-newkey', 'rsa:2048',
        '-subj', '/CN=localhost', '-keyout', str(key), '-out', str(cert))
    with service(candidate / 'serve.py', list(map(str, [root, cert, key]))) as port:
        url = f'https://127.0.0.1:{port}'
        for branch, prefix in [('main', ''), ('dev', '/dev')]:
            assert response(url + prefix + '/index.html') == (200, branch.encode() + b'\x00\xff\n')
            assert response(url + prefix + '/nested/data.bin') == (200, branch.encode() + b' nested\x00')
            assert response(url + prefix + '/missing')[0] == 404
            (root / branch / 'index.html').write_bytes(b'updated ' + branch.encode())
            assert response(url + prefix + '/index.html') == (200, b'updated ' + branch.encode())


def main():
    candidate, oracle = (Path(p).resolve() for p in sys.argv[1:])
    child = json.loads((oracle / 'task.json').read_text())['child']
    assert child in ('m1', 'm2', 'm3', 'm4')
    with tempfile.TemporaryDirectory(prefix='multibranch probe ', dir=candidate) as directory:
        root = Path(directory)
        if child == 'm1':
            check_auth(candidate, root)
        elif child == 'm4':
            check_https(candidate, root)
        else:
            check_deploy(candidate, root, child)


if __name__ == '__main__':
    try:
        main()
    except (AssertionError, OSError, subprocess.TimeoutExpired, urllib.error.URLError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
