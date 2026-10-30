"""Black-box Python API, valid wheel, live HTTP and isolated pip checks."""
import base64
import csv
import functools
import hashlib
import http.server
import io
import json
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
import zipfile

PROBE = '''import sys,math,importlib,importlib.metadata
from pathlib import Path
root=Path(sys.argv[1]).resolve()
sys.path.insert(0,str(root))
m=importlib.import_module(sys.argv[2])
assert Path(m.__file__).resolve().is_relative_to(root)
for xs in ([],[3],[-3,4],[0.5,-1.25,2],[2**30,1,-2]):
    before=xs.copy()
    result=m.norm2(xs)
    assert type(result) in (int,float) and math.isclose(result,sum(x*x for x in xs),rel_tol=1e-12)
    assert xs==before
if len(sys.argv)>3:
    assert importlib.metadata.version(sys.argv[2])=='0.4.2'
'''


def run(args, cwd, timeout=25):
    return subprocess.run([sys.executable, *args], cwd=cwd, capture_output=True,
                          timeout=timeout).returncode == 0


def probe(root, name, installed=False):
    return run(['-I', '-c', PROBE, str(root), name] + (['installed'] if installed else []), root)


def install(app, wheel, target):
    return run(['-m', 'pip', '--isolated', 'install', '--no-index', '--no-deps',
                '--no-cache-dir', '--target', str(target), str(wheel)], app)


def valid_wheel(path):
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        prefix = 'coppervec-0.4.2.dist-info/'
        if len(names) != len(set(names)) or 'coppervec/__init__.py' not in names:
            return False
        metadata = archive.read(prefix + 'METADATA').decode().splitlines()
        if 'Name: coppervec' not in metadata or 'Version: 0.4.2' not in metadata:
            return False
        if 'Tag: py3-none-any' not in archive.read(prefix + 'WHEEL').decode().splitlines():
            return False
        record = prefix + 'RECORD'
        rows = list(csv.reader(io.StringIO(archive.read(record).decode())))
        if len(rows) != len(names) or {row[0] for row in rows} != set(names):
            return False
        for name, digest, size in rows:
            if name == record:
                if digest or size:
                    return False
            else:
                data = archive.read(name)
                expected = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).decode().rstrip('=')
                if digest != 'sha256=' + expected or size != str(len(data)):
                    return False
        return True


def free_port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


def get(url):
    # No proxy or redirects may move the checker outside loopback.
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, newurl):
            return None
    return urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect()).open(url, timeout=2)


def server_check(app):
    if not (app / 'serve.py').is_file():
        return False
    port = free_port()
    root = app / 'index'
    process = subprocess.Popen([sys.executable, 'serve.py', str(port), str(root)],
                               cwd=app, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        base = f'http://127.0.0.1:{port}'
        for _ in range(40):
            if process.poll() is not None:
                return False
            try:
                with get(base + '/simple/ambervec/') as response:
                    if response.read() != (root / 'simple/ambervec/index.html').read_bytes():
                        return False
                break
            except (OSError, urllib.error.URLError):
                time.sleep(0.05)
        else:
            return False
        name = 'ambervec-0.4.2-py3-none-any.whl'
        with get(base + '/packages/' + name) as response:
            if response.read() != (root / 'packages' / name).read_bytes():
                return False
        try:
            get(base + '/simple/absent-project/')
        except urllib.error.HTTPError as error:
            return error.code == 404
        return False
    finally:
        if process.poll() is None:
            process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill(); process.wait()


class IndexHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        self.server.requests.append(self.path)
        super().do_GET()


def installation_check(app):
    if not (app / 'install.py').is_file():
        return False
    with tempfile.TemporaryDirectory() as tmp:
        # Copy public wheel bytes into a fresh verifier-controlled index root.
        root = Path(tmp) / 'index'
        import shutil
        shutil.copytree(app / 'index', root)
        target = Path(tmp) / 'installed'
        with http.server.ThreadingHTTPServer(('127.0.0.1', 0), functools.partial(IndexHandler, directory=root)) as server:
            server.requests = []
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                url = f'http://127.0.0.1:{server.server_port}/simple'
                if not run(['install.py', url, str(target)], app):
                    return False
                paths = [path.split('?', 1)[0] for path in server.requests]
                return ('/simple/ambervec/' in paths
                        and '/packages/ambervec-0.4.2-py3-none-any.whl' in paths
                        and probe(target, 'ambervec', True))
            finally:
                server.shutdown(); thread.join(timeout=3)


def check(app, oracle):
    kind = json.loads((oracle / 'expected.json').read_text())['id']
    try:
        if kind == 'm1':
            return (app / 'tinyvec/__init__.py').is_file() and probe(app, 'tinyvec')
        if kind == 'm2':
            if not (app / 'build.py').is_file() or not run(['build.py'], app):
                return False
            wheels = list((app / 'dist').glob('*.whl'))
            if len(wheels) != 1 or wheels[0].name != 'coppervec-0.4.2-py3-none-any.whl' or not valid_wheel(wheels[0]):
                return False
            with tempfile.TemporaryDirectory() as tmp:
                target = Path(tmp)
                return install(app, wheels[0], target) and probe(target, 'coppervec', True)
        if kind == 'm3':
            return server_check(app)
        if kind == 'm4':
            return installation_check(app)
        raise ValueError('unknown diagnostic')
    except (OSError, ValueError, KeyError, zipfile.BadZipFile, subprocess.TimeoutExpired):
        return False


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()) else 1)
