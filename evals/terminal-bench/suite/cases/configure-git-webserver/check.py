"""Execute submitted setup/server programs, then independently probe Git and HTTP."""
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request


def run(*args, cwd=None):
    result = subprocess.run(args, cwd=cwd, env={**os.environ, 'GIT_CONFIG_NOSYSTEM': '1',
                            'GIT_CONFIG_GLOBAL': '/dev/null'}, capture_output=True, timeout=15)
    assert result.returncode == 0, result.stderr.decode(errors='replace')
    return result.stdout


def git(path, *args):
    return run('git', '-C', str(path), *args)


def commit(path, label):
    git(path, 'add', '-A')
    git(path, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
        'commit', '-m', label)
    git(path, 'push', 'origin', 'HEAD')


def setup(candidate, root):
    script = candidate / 'setup.sh'
    assert script.is_file(), 'setup.sh missing'
    run('sh', str(script.resolve()), str(root))
    server = root / 'server.git'
    assert git(server, 'rev-parse', '--is-bare-repository').strip() == b'true'
    assert git(server, 'symbolic-ref', 'HEAD').strip() == b'refs/heads/master'
    assert not git(server, 'for-each-ref').strip(), 'repository must start empty'
    return server


def check_git(candidate, child, directory):
    root = directory / 'deployment space'
    root.mkdir()
    private = root / 'private.txt'
    private.write_bytes(b'private sentinel\x00\xff\n')
    private.chmod(0o600)
    server = setup(candidate, root)
    client = directory / 'client'
    run('git', 'clone', str(server), str(client))
    content = b'hello synthetic site\nsecond line\x00\xff'
    (client / 'hello.html').write_bytes(content)
    (client / 'removed.txt').write_bytes(b'remove in next revision\n')
    commit(client, 'initial synthetic site')
    if child == 'm1':
        clone = directory / 'second clone'
        run('git', 'clone', str(server), str(clone))
        assert (clone / 'hello.html').read_bytes() == content
        assert git(clone, 'rev-parse', 'HEAD') == git(client, 'rev-parse', 'HEAD')
        return
    public = root / 'public'
    assert (public / 'hello.html').read_bytes() == content
    if child == 'm2':
        git(client, 'checkout', '-b', 'preview')
        (client / 'hello.html').write_bytes(b'not the production branch\n')
        commit(client, 'preview only')
        assert (public / 'hello.html').read_bytes() == content, 'side branch deployed'
        git(client, 'checkout', 'master')
    changed = b'revision two\n'
    (client / 'hello.html').write_bytes(changed)
    (client / 'removed.txt').unlink()
    commit(client, 'update and remove')
    assert (public / 'hello.html').read_bytes() == changed
    assert not (public / 'removed.txt').exists()
    if child == 'm3':
        assert private.read_bytes() == b'private sentinel\x00\xff\n'
        assert private.stat().st_mode & 0o777 == 0o600
        assert not (public / '.git').exists()
        for path in [root, *root.rglob('*')]:
            assert not path.is_symlink(), 'deployment must not use escaping links'
            assert not path.stat().st_mode & 0o002, 'world-writable generated artifact'


def response(port, path):
    try:
        with urllib.request.urlopen(f'http://127.0.0.1:{port}{path}', timeout=2) as result:
            return result.status, result.read()
    except urllib.error.HTTPError as error:
        return error.code, error.read()


def check_http(candidate, directory):
    script = candidate / 'serve.py'
    assert script.is_file(), 'serve.py missing'
    public = directory / 'public'
    public.mkdir()
    (public / 'nested').mkdir()
    (public / '.git').mkdir()
    (public / '.git/config').write_bytes(b'PRIVATE-GIT-CONFIG')
    private = directory / 'private.txt'
    private.write_bytes(b'PRIVATE-SENTINEL')
    (public / 'escape.txt').symlink_to(private)
    payload = b'page from fixture\n\xc3\xa9\x00'
    (public / 'hello.html').write_bytes(payload)
    (public / 'nested/page.txt').write_bytes(b'nested page\n')
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    process = subprocess.Popen([sys.executable, str(script.resolve()), str(public), str(port)],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(60):
            assert process.poll() is None, 'server exited'
            try:
                actual = response(port, '/hello.html')
                break
            except (OSError, urllib.error.URLError):
                time.sleep(0.05)
        else:
            raise AssertionError('server did not start')
        assert actual == (200, payload)
        assert response(port, '/nested/page.txt') == (200, b'nested page\n')
        assert response(port, '/missing.txt')[0] == 404
        for path in ('/.git/config', '/%2egit/config', '/escape.txt'):
            assert response(port, path)[0] == 404, 'private path exposed'
        (public / 'hello.html').write_bytes(b'updated HTTP bytes\n')
        assert response(port, '/hello.html') == (200, b'updated HTTP bytes\n')
    finally:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def main():
    candidate, oracle = map(Path, sys.argv[1:])
    child = json.loads((oracle / 'task.json').read_text())['child']
    assert child in ('m1', 'm2', 'm3', 'm4')
    with tempfile.TemporaryDirectory(prefix='git-web-probe-', dir=candidate.resolve()) as name:
        if child == 'm4':
            check_http(candidate, Path(name))
        else:
            check_git(candidate, child, Path(name))


if __name__ == '__main__':
    try:
        main()
    except (AssertionError, OSError, subprocess.TimeoutExpired) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
