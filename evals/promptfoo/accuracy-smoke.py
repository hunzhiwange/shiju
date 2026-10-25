#!/usr/bin/env python3
"""Trusted single-attempt supervisor. JSON-lines stdin/stdout; secrets stay in memory."""
import hashlib
import http.server
import importlib.util
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import socket
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
import urllib.error
from datetime import datetime, timezone

_SPEC = importlib.util.spec_from_file_location('preflight', Path(__file__).with_name('gateway-preflight.py'))
preflight = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(preflight)
REPO = Path(__file__).resolve().parents[2]
RESULTS = REPO / 'evals/promptfoo/results/accuracy-smoke'
MODEL = 'deepseek-v4-flash-0731'


def sha(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


class ModelGate:
    def __init__(self, model):
        self.model, self.calls, self.enabled = model, 0, False
        self.busy = False
        self.lock = threading.Lock()

    def accept(self, path, payload):
        with self.lock:
            body = json.loads(payload)
            if not self.enabled or path != '/v1/chat/completions' or self.calls >= 24 or self.busy:
                raise ValueError('model channel closed or budget exceeded')
            if body.get('model') != self.model or not isinstance(body.get('messages'), list):
                raise ValueError('model channel identity mismatch')
            self.calls += 1
            self.busy = True
            return body

    def finish(self, success):
        with self.lock:
            self.busy = False
            if not success:
                self.enabled = False


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def local_model():
    # Exact existing profile, not all host config or environment. Never copy the
    # auth file, entire provider object or host headers into the sandbox.
    root = Path.home() / '.shiju-agent'
    database = root / 'database/vibe-window.sqlite3'
    with sqlite3.connect(f'file:{database}?mode=ro', uri=True) as connection:
        rows = connection.execute('SELECT payload_json FROM system_configs').fetchall()
    endpoints = set()
    for (raw,) in rows:
        provider = json.loads(raw).get('providers', {}).get('token-plan', {})
        if MODEL in provider.get('models', {}):
            endpoints.add(provider.get('api'))
    if len(endpoints) != 1:
        raise RuntimeError('existing token-plan model endpoint missing or ambiguous')
    endpoint = endpoints.pop()
    parsed = urllib.parse.urlsplit(endpoint)
    if parsed.scheme != 'https' or parsed.username or parsed.password or parsed.query or parsed.fragment:
        raise RuntimeError('existing model endpoint is not credential-free HTTPS')
    auth = json.loads((root / 'auth.json').read_text()).get('token-plan', {})
    key = auth.get('key') if auth.get('type') == 'api' else None
    if not isinstance(key, str) or not key or any(ord(c) < 33 or ord(c) > 126 for c in key):
        raise RuntimeError('existing token-plan API credential unavailable')
    return endpoint.rstrip('/') + '/chat/completions', key


def profile(runtime, work, port, relay):
    q = json.dumps
    return '\n'.join([
        '(version 1)', '(deny default)', '(allow process-exec)',
        # No fork means no shell/background/detached writers can outlive Gateway.
        '(allow sysctl-read (sysctl-name-regex #"^hw[.]")',
        ' (sysctl-name "kern.osrelease") (sysctl-name "kern.osversion")',
        ' (sysctl-name "kern.ostype") (sysctl-name "kern.version"))',
        '(allow file-read-metadata (literal "/private") (literal "/private/tmp")',
        f' (literal "/usr") (literal "/System") (literal {q(runtime)}) ' +
        ' '.join(f'(literal {q(str(parent))})' for parent in Path(work).parents) + ')',
        '(allow file-read* (literal "/") (literal "/private/etc/ssl/openssl.cnf") (subpath "/System/Library")',
        ' (subpath "/usr/lib") (subpath "/usr/bin") (subpath "/bin")',
        ' (literal "/dev/null") (literal "/dev/random") (literal "/dev/urandom")',
        f' (subpath {q(runtime + "/bin")}) (subpath {q(work)}) (subpath {q(runtime + "/state")}))',
        f'(allow file-write* (subpath {q(work)}) (subpath {q(runtime + "/state")}) (literal "/dev/null"))',
        f'(allow network-bind (local ip "localhost:{port}"))',
        f'(allow network-inbound (local ip "localhost:{port}"))',
        f'(allow network-outbound (remote ip "localhost:{relay}"))',
    ])


def check_model_catalogue(catalogue, api):
    providers = [p for p in catalogue.get('all', []) if p.get('id') == 'token-plan']
    if len(providers) != 1:
        raise RuntimeError('runtime model provider unavailable before admission')
    model = providers[0].get('models', {}).get(MODEL, {})
    if model.get('api', {}).get('url') != api or model.get('api', {}).get('id') != MODEL:
        raise RuntimeError('runtime model route unavailable before admission')


class Supervisor:
    def __init__(self):
        self.lock = self.process = self.build = self.runtime = self.server = None
        self.used = False
        self.gate = ModelGate(MODEL)
        self.evidence = {'commands': [], 'canaries': [], 'modelTasks': 0, 'status': 'preparing'}

    def initialize(self):
        if platform.system() != 'Darwin':
            raise RuntimeError('macOS Seatbelt required')
        RESULTS.mkdir(parents=True, exist_ok=True)
        if (RESULTS / 'real-attempt-started').exists():
            raise RuntimeError('single real attempt already reserved; no automatic retry')
        self.upstream, self.key = local_model()
        locks = REPO / 'target/rust-slot-locks'
        locks.mkdir(parents=True, exist_ok=True)
        while self.lock is None:
            for slot in range(1, 11):
                candidate = locks / f'slot-{slot}.lock'
                try:
                    candidate.mkdir()
                except FileExistsError:
                    continue
                self.lock = candidate
                break
            if self.lock is None:
                time.sleep(2)
        (self.lock / 'owner').write_text(json.dumps({'plan': '底层准确度评测', 'task': 20,
            'session': 'accuracy-smoke', 'pid': os.getpid()}))
        target = REPO / f'target/rust-slot-{slot}'
        self.evidence.update(slot=slot, CARGO_TARGET_DIR=str(target),
            upstreamEndpoint=self.upstream, model='token-plan/' + MODEL,
            sourceCommit=preflight.run(['git', 'rev-parse', 'HEAD'], cwd=REPO).stdout.strip(),
            sourceDigest=preflight.source_digest(REPO))
        command = ['cargo', 'build', '-p', 'vw-cli', '--bin', 'shiju']
        with (RESULTS / 'build.log').open('w') as log:
            self.build = subprocess.Popen(command, cwd=REPO,
                env=dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target)), stdout=log, stderr=log,
                start_new_session=True)
            self.build.wait()
        self.evidence['commands'].append({'argv': command, 'exitCode': self.build.returncode})
        if self.build.returncode:
            raise RuntimeError('build failed; see ignored build.log')
        if self.evidence['sourceDigest'] != preflight.source_digest(REPO):
            raise RuntimeError('Rust/Cargo sources changed during build')
        self.runtime = Path(tempfile.mkdtemp(prefix='vw-smoke-', dir='/private/tmp'))
        for name in ('bin', 'state', 'forbidden'):
            (self.runtime / name).mkdir(mode=0o700)
        self.binary = self.runtime / 'bin/shiju'
        result = preflight.run(['/bin/cp', '-c', str(target / 'debug/shiju'), str(self.binary)])
        if result.returncode:
            raise RuntimeError('APFS clone failed')
        self.binary.chmod(0o500)
        self.evidence['binarySha256'] = preflight.digest(self.binary)
        owner = self

        class Relay(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_POST(self):
                try:
                    length = int(self.headers.get('Content-Length', '0'))
                    if not 0 < length <= 8 * 1024 * 1024:
                        raise ValueError('invalid request size')
                    payload = self.rfile.read(length)
                    owner.gate.accept(self.path, payload)
                except (ValueError, json.JSONDecodeError):
                    self.send_error(403, 'restricted model channel')
                    return
                request = urllib.request.Request(owner.upstream, data=payload,
                    headers={'Content-Type': 'application/json', 'Authorization': 'Bearer ' + owner.key})
                opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
                try:
                    with opener.open(request, timeout=120) as response:
                        self.send_response(response.status)
                        self.send_header('Content-Type', response.headers.get('Content-Type', 'text/event-stream'))
                        self.end_headers()
                        # Hold a secret-length tail to redact even across read boundaries.
                        pending = b''
                        secret = owner.key.encode()
                        while chunk := response.read1(65536):
                            pending += chunk
                            pending = pending.replace(secret, b'[redacted]')
                            count = max(0, len(pending) - len(secret) + 1)
                            self.wfile.write(pending[:count])
                            pending = pending[count:]
                            self.wfile.flush()
                        self.wfile.write(pending.replace(secret, b'[redacted]'))
                        self.wfile.flush()
                    owner.gate.finish(True)
                except Exception:
                    owner.gate.finish(False)
                    owner.evidence['relayError'] = 'upstream model transport failed; not retried'
                    self.close_connection = True

        self.server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Relay)
        self.server.daemon_threads = False
        self.relay_thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.relay_thread.start()
        self.relay = self.server.server_address[1]
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0))
            self.port = sock.getsockname()[1]
        self.endpoint = f'http://127.0.0.1:{self.port}'
        return {'endpoint': self.endpoint + '/v1/chat/stream'}

    def request(self, route, data=None, scoped=True):
        request = urllib.request.Request(self.endpoint + route,
            data=None if data is None else json.dumps(data).encode(),
            method='GET' if data is None else 'PATCH',
            headers={'Content-Type': 'application/json', 'X-Platform-ID': '10000',
                     'X-Company-ID': '100000000', **({'X-VibeWindow-Directory': self.work} if scoped else {})})
        with urllib.request.urlopen(request, timeout=10) as response:
            return json.load(response)

    def admit(self, attemptId, workRoot):
        if self.used:
            raise RuntimeError('only one attempt permitted')
        self.used = True
        self.work = workRoot
        work = Path(workRoot)
        if work.is_symlink() or not work.is_dir() or work.resolve() != work or not work.name.startswith('vw-accuracy-') or work.parent != Path(tempfile.gettempdir()).resolve():
            raise RuntimeError('untrusted workRoot')
        self.evidence.update(attemptId=attemptId, workRoot=workRoot,
            workRootIdentity={'dev': work.stat().st_dev, 'ino': work.stat().st_ino},
            verifiedAt=datetime.now(timezone.utc).isoformat(), endpoint=self.endpoint)
        forbidden = self.runtime / 'forbidden'
        (forbidden / 'sentinel').write_text('disposable-canary\n')
        profile_path = RESULTS / 'gateway.sb'
        profile_path.write_text(profile(str(self.runtime), workRoot, self.port, self.relay))
        self.prefix = ['/usr/bin/sandbox-exec', '-f', str(profile_path)]
        state = self.runtime / 'state'
        self.env = {'PATH': '/usr/bin:/bin', 'HOME': str(state), 'TMPDIR': str(state),
            'VIBEWINDOW_CONFIG_DIR': str(state / 'config'), 'VIBEWINDOW_TEST_HOME': str(state),
            'CARGO_TARGET_DIR': self.evidence['CARGO_TARGET_DIR'],
            'CARGO_BUILD_BUILD_DIR': self.evidence['CARGO_TARGET_DIR'], 'RUST_LOG': 'warn',
            'LANG': 'en_US.UTF-8', 'OPENSSL_CONF': '/dev/null', 'VW_SMOKE_KEY': 'channel-only-not-a-secret'}
        (work / 'escape').symlink_to(forbidden, target_is_directory=True)
        checks = [
            ('allowed-write', ['/bin/sh', '-c', 'printf ok > canary'], True),
            ('allowed-read', ['/bin/cat', str(work / 'canary')], True),
            ('absolute-read-denied', ['/bin/cat', str(forbidden / 'sentinel')], False),
            ('absolute-write-denied', ['/bin/sh', '-c', f'printf bad > {forbidden}/written'], False),
            ('symlink-read-denied', ['/bin/cat', str(work / 'escape/sentinel')], False),
            ('symlink-write-denied', ['/bin/sh', '-c', 'printf bad > escape/written'], False),
            ('repo-denied', ['/bin/cat', str(REPO / 'Cargo.toml')], False),
            ('evidence-denied', ['/bin/cat', str(profile_path)], False),
            ('binary-write-denied', ['/bin/sh', '-c', f'printf bad > {self.binary}'], False),
            ('volume-alias-denied', ['/bin/cat', '/System/Volumes/Data' + str(forbidden / 'sentinel')], False),
            ('fork-denied', ['/usr/bin/perl', '-e', 'defined(fork()) or die "$!\\n";'], False),
        ]
        for name, command, allowed in checks:
            result = preflight.run(self.prefix + command, cwd=work, env=self.env)
            passed = result.returncode == 0 if allowed else result.returncode != 0 and 'Operation not permitted' in result.stderr
            self.evidence['canaries'].append({'name': name, 'exitCode': result.returncode, 'pass': passed})
            if not passed:
                raise RuntimeError('canary failed: ' + name)
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            listener.listen()
            command = ['/usr/bin/curl', '-v', '--noproxy', '*', '--max-time', '2',
                f'http://127.0.0.1:{listener.getsockname()[1]}']
            result = preflight.run(self.prefix + command, cwd=work, env=self.env)
            listener.settimeout(0.1)
            try:
                conn, _ = listener.accept()
                conn.close()
                raise RuntimeError('forbidden network accepted')
            except socket.timeout:
                pass
            passed = result.returncode != 0 and 'Operation not permitted' in result.stderr
            self.evidence['canaries'].append({'name': 'other-network-denied', 'pass': passed, 'exitCode': result.returncode, 'stderr': result.stderr})
            if not passed:
                raise RuntimeError('network denial not established')
        # Valid relay route is still closed before admission: no model charge.
        result = preflight.run(self.prefix + ['/usr/bin/curl', '--noproxy', '*', '--max-time', '3',
            '-s', '-o', '/dev/null', '-w', '%{http_code}', '-X', 'POST', '-d', '{}',
            f'http://127.0.0.1:{self.relay}/v1/chat/completions'], cwd=work, env=self.env)
        passed = result.returncode == 0 and result.stdout == '403' and self.gate.calls == 0
        self.evidence['canaries'].append({'name': 'relay-reachable-closed', 'pass': passed, 'exitCode': result.returncode, 'stderr': result.stderr, 'stdout': result.stdout})
        if not passed:
            raise RuntimeError('restricted relay connectivity failed')
        (work / 'escape').unlink()
        (work / 'canary').unlink()
        if (forbidden / 'written').exists() or (forbidden / 'sentinel').read_text() != 'disposable-canary\n':
            raise RuntimeError('forbidden canary changed')
        command = self.prefix + [str(self.binary), 'gateway', '--host', '127.0.0.1', '--port', str(self.port)]
        self.evidence['launchCommand'] = command
        with (RESULTS / 'gateway.log').open('w') as log:
            self.process = subprocess.Popen(command, cwd=work, env=self.env, stdout=log, stderr=log, start_new_session=True)
        for _ in range(60):
            if self.process.poll() is not None:
                raise RuntimeError('Gateway exited during startup')
            try:
                self.request('/v1/health')
                break
            except OSError:
                time.sleep(0.5)
        else:
            raise RuntimeError('Gateway health timeout')
        provider = {'api': f'http://127.0.0.1:{self.relay}/v1', 'adapter': 'openai-compatible',
            'env': ['VW_SMOKE_KEY'], 'models': {MODEL: {'id': MODEL, 'status': 'active'}}}
        patch = {'providers': {'token-plan': provider}, 'enabled_providers': ['token-plan'],
            'default_provider': 'token-plan', 'default_model': 'token-plan/' + MODEL,
            'autonomy': {'level': 'full', 'workspace_only': True, 'allowed_roots': [],
                'allowed_commands': [], 'block_high_risk_commands': True},
            'security': {'bash': {'enabled': True}}, 'reliability': {'provider_retries': 0}}
        # Match the runtime's default tenant, including factories loading global
        # config outside a task-local request context. Both live only in this HOME.
        self.request('/v1/config', patch, scoped=False)
        self.request('/v1/config', patch)
        config = self.request('/v1/config')
        if config['providers']['token-plan']['api'] != provider['api'] or not config['autonomy']['workspace_only'] or config['autonomy']['allowed_roots']:
            raise RuntimeError('effective config mismatch')
        catalogue = self.request('/v1/provider')
        (RESULTS / 'model-catalogue.json').write_text(json.dumps(catalogue))
        check_model_catalogue(catalogue, provider['api'])
        tools = self.request('/v1/tools')
        (RESULTS / 'config-effective.json').write_text(json.dumps(config))
        (RESULTS / 'tools-effective.json').write_text(json.dumps(tools))
        self.evidence.update(configurationDigest=sha({'config': config, 'tools': tools,
            'profile': preflight.digest(profile_path), 'binary': self.evidence['binarySha256'],
            'workRoot': workRoot, 'uid': os.getuid()}), forbiddenWriteScopes=[str(REPO), str(Path.home()), str(forbidden), str(RESULTS)])
        proof = {key: self.evidence[key] for key in ('workRoot', 'configurationDigest', 'forbiddenWriteScopes', 'verifiedAt')}
        proof.update(mode='restricted-sandbox', evidencePath=str(RESULTS / 'environment.json'))
        proof['digest'] = sha({'proof': proof, 'attemptId': attemptId, 'canaries': self.evidence['canaries']})
        self.proof = proof
        self.evidence['status'] = 'admitted-channel-closed'
        self.save()
        return proof

    def arm(self):
        if self.evidence.get('status') != 'admitted-channel-closed':
            raise RuntimeError('channel requires fresh admission')
        # Durable no-retry marker before any model work can begin.
        with (RESULTS / 'real-attempt-started').open('x') as marker:
            marker.write(self.evidence['attemptId'])
        self.evidence['modelTasks'] = 1
        self.gate.enabled = True
        self.evidence['status'] = 'admitted'
        self.save()
        return {'armed': True}

    def stop(self):
        self.gate.enabled = False
        if self.process is not None:
            try:
                os.killpg(self.process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(self.process.pid, signal.SIGKILL)
                self.process.wait(timeout=5)
            try:
                os.killpg(self.process.pid, 0)
                raise RuntimeError('Gateway process group remains; retain slot')
            except ProcessLookupError:
                pass
            with socket.socket() as sock:
                if sock.connect_ex(('127.0.0.1', self.port)) == 0:
                    raise RuntimeError('Gateway port remains open')
            self.evidence.update(writersExited=True, gatewayExitCode=self.process.returncode, endpointClosed=True)

    def settle(self, attemptId, workRoot, admission, sessionId=None):
        if attemptId != self.evidence['attemptId'] or workRoot != self.work or admission != self.proof:
            raise RuntimeError('settlement identity mismatch')
        self.stop()
        work = Path(self.work)
        identity = self.evidence['workRootIdentity']
        if work.is_symlink() or work.stat().st_dev != identity['dev'] or work.stat().st_ino != identity['ino']:
            raise RuntimeError('workspace identity changed')
        archive = RESULTS / 'artifacts'
        archive.mkdir(exist_ok=False)
        names = sorted(p.name for p in work.iterdir())
        for name in names:
            path = work / name
            if path.is_symlink() or not path.is_file():
                raise RuntimeError('unexpected nonregular artifact')
            shutil.copyfile(path, archive / name)
        self.evidence.update(sessionId=sessionId, status='settled', artifactFiles=names)
        self.save()
        return {'writersExited': True, 'admissionDigest': admission['digest'],
            'checks': [{'kind': 'non_target_files', 'pass': names == ['notes.txt', 'settings.ini'] and
                (work / 'notes.txt').read_bytes() == (REPO / 'evals/promptfoo/fixtures/runtime-accuracy/edit/edit-01/notes.txt').read_bytes()},
                {'kind': 'forbidden_canary', 'pass': not (self.runtime / 'forbidden/written').exists()}]}

    def save(self):
        self.evidence['modelCalls'] = self.gate.calls
        (RESULTS / 'environment.json').write_text(json.dumps(self.evidence, indent=2, ensure_ascii=False) + '\n')

    def close(self):
        self.stop()
        if self.server:
            self.server.shutdown()
            self.server.server_close()
        if self.build and self.build.poll() is None:
            os.killpg(self.build.pid, signal.SIGKILL)
            self.build.wait()
        if self.runtime:
            shutil.rmtree(self.runtime)
        if self.lock:
            (self.lock / 'owner').unlink()
            self.lock.rmdir()
            self.evidence['slotReleased'] = True
        if RESULTS.exists():
            self.save()
            (RESULTS / f'environment-{os.getpid()}.json').write_text(json.dumps(self.evidence, indent=2))


def main():
    supervisor = Supervisor()
    def interrupted(signum, _frame):
        raise KeyboardInterrupt(signum)
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    try:
        for line in sys.stdin:
            try:
                request = json.loads(line)
                method = request['method']
                if method not in ('initialize', 'admit', 'arm', 'settle', 'close'):
                    raise RuntimeError('unknown supervisor method')
                result = getattr(supervisor, method)(**request.get('args', {}))
                print(json.dumps({'result': result}), flush=True)
                if method == 'close':
                    return 0
            except Exception as error:
                # Only our fixed errors are reported; credential/file/HTTP exceptions
                # may contain sensitive data and are deliberately reduced to type.
                message = str(error) if isinstance(error, RuntimeError) else type(error).__name__
                supervisor.evidence['error'] = message
                print(json.dumps({'error': message}), flush=True)
                return 1
    finally:
        if not supervisor.evidence.get('slotReleased'):
            supervisor.close()
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
