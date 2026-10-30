"""Start real Nginx; compare fresh HTTP responses and appended access records."""
import json
from pathlib import Path
import re
import socket
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request


def run(*args):
    result = subprocess.run(args, capture_output=True, timeout=10)
    assert result.returncode == 0, result.stderr.decode(errors='replace')


def response(port, path, agent, method='GET'):
    request = urllib.request.Request(f'http://127.0.0.1:{port}{path}',
                                     headers={'User-Agent': agent}, method=method)
    try:
        with urllib.request.urlopen(request, timeout=2) as result:
            return result.status, result.read()
    except urllib.error.HTTPError as error:
        return error.code, error.read()


def check(candidate, child, root):
    (root / 'public/nested').mkdir(parents=True)
    (root / 'public/index.html').write_bytes(b'synthetic home\n')
    (root / 'public/nested/item.txt').write_bytes(b'nested fixture\n')
    (root / 'public/404.html').write_bytes(b'synthetic missing page\n')
    sentinel = 'preexisting access sentinel\n'
    (root / 'access.log').write_text(sentinel)
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    if child == 'm4':
        # This configuration is verifier-owned and never exposed as a fixture.
        (root / 'nginx.conf').write_text(
            f'user root; pid {root}/nginx.pid; error_log {root}/error.log; events {{}} '
            f'http {{ client_body_temp_path {root}/client-body; proxy_temp_path {root}/proxy; '
            f'fastcgi_temp_path {root}/fastcgi; uwsgi_temp_path {root}/uwsgi; '
            f'scgi_temp_path {root}/scgi; log_format probe \'$request_method|$uri|$status|$http_user_agent\'; '
            f'access_log {root}/access.log probe; server {{ listen 127.0.0.1:{port}; '
            f'root {root}/public; }} }}')
    else:
        script = candidate / 'configure.py'
        assert script.is_file(), 'configure.py missing'
        run(sys.executable, str(script), str(root), str(port))
    config = root / 'nginx.conf'
    assert config.is_file(), 'nginx.conf missing'
    run('nginx', '-t', '-p', str(root), '-c', str(config))
    process = subprocess.Popen(['nginx', '-p', str(root), '-c', str(config),
                               '-g', 'daemon off;'], stdout=subprocess.DEVNULL,
                              stderr=subprocess.DEVNULL)
    try:
        for _ in range(60):
            assert process.poll() is None, 'nginx exited'
            try:
                with socket.create_connection(('127.0.0.1', port), timeout=0.1):
                    break
            except OSError:
                time.sleep(0.05)
        else:
            raise AssertionError('nginx did not listen')
        if child == 'm4':
            script = candidate / 'request.py'
            assert script.is_file(), 'request.py missing'
            agent = 'SyntheticProbe/63-alpha'
            run(sys.executable, str(script), f'http://127.0.0.1:{port}', agent)
            expected = [f'GET|/index.html|200|{agent}', f'GET|/absent|404|{agent}']
        elif child == 'm1':
            assert response(port, '/index.html', 'probe') == (200, b'synthetic home\n')
            assert response(port, '/nested/item.txt', 'probe') == (200, b'nested fixture\n')
            assert response(port, '/missing', 'probe')[0] == 404
            (root / 'public/index.html').write_bytes(b'updated bytes\n')
            assert response(port, '/index.html', 'probe') == (200, b'updated bytes\n')
        elif child == 'm2':
            for path, agent, method in [('/index.html','Agent-A','GET'),
                                        ('/absent','Agent-B','GET'),
                                        ('/index.html','Agent-C','HEAD'),
                                        ('/index.html','quote"slash\\probe','GET')]:
                response(port, path, agent, method)
            expected = [('GET', '200', 'Agent-A'), ('GET', '404', 'Agent-B'),
                        ('HEAD', '200', 'Agent-C'), ('GET', '200', r'quote\x22slash\x5Cprobe')]
        else:
            for path in ('/absent', '/nested/missing?query=1', '/404.html'):
                assert response(port, path, 'boundary') == (404, b'synthetic missing page\n')
            assert (root / 'error.log').is_file(), 'error log missing'
        # Graceful termination flushes all logs before observation.
        process.terminate()
        process.wait(timeout=5)
        if child in ('m2', 'm3', 'm4'):
            log = (root / 'access.log').read_text()
            assert log.startswith(sentinel), 'existing access content lost'
            lines = log[len(sentinel):].splitlines()
            if child == 'm4':
                assert lines == expected, f'unexpected request sequence: {lines!r}'
            elif child == 'm2':
                assert len(lines) == len(expected), 'one record per request required'
                pattern = r'\d{2}/[A-Za-z]{3}/\d{4}:\d{2}:\d{2}:\d{2} [+-]\d{4} \| ([A-Z]+) \| (\d{3}) \| "(.*)"'
                observed = []
                for line in lines:
                    match = re.fullmatch(pattern, line)
                    assert match, f'malformed record: {line!r}'
                    observed.append(match.groups())
                assert observed == expected, 'request fields mismatch'
            else:
                assert len(lines) == 3, 'missing boundary request logs'
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


def main():
    candidate, oracle = [Path(value).resolve() for value in sys.argv[1:]]
    child = json.loads((oracle / 'task.json').read_text())['child']
    assert child in ('m1', 'm2', 'm3', 'm4')
    with tempfile.TemporaryDirectory(prefix='nginx-probe-', dir=candidate) as directory:
        check(candidate, child, Path(directory))


if __name__ == '__main__':
    if shutil.which('nginx') is None:
        print('infrastructure: nginx is not installed', file=sys.stderr)
        sys.exit(2)
    try:
        main()
    except (AssertionError, OSError, subprocess.TimeoutExpired) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
