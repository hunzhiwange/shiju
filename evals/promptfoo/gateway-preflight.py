#!/usr/bin/env python3
"""Run a credential-free macOS Gateway and inherited Seatbelt canaries.

This checks startup only. It never submits a model request or admits a later
attempt: changing workRoot, configuration, tools or networking requires a rerun.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import socket
import subprocess
import tempfile
import time
import urllib.request
import urllib.error
from datetime import datetime, timezone


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(args, **kwargs):
    return subprocess.run(args, capture_output=True, text=True, timeout=30, **kwargs)


def source_digest(repo):
    paths = subprocess.check_output(['git', 'ls-files', '-co', '--exclude-standard', '-z'], cwd=repo)
    hasher = hashlib.sha256()
    for name in sorted(set(paths.decode().split('\0'))):
        if name and (name.endswith('.rs') or Path(name).name in ('Cargo.toml', 'Cargo.lock')):
            path = repo / name
            if path.is_file():
                hasher.update(name.encode() + b'\0' + path.read_bytes() + b'\0')
    return hasher.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reuse-build', action='store_true',
                        help='Use an already built default-feature binary in the acquired slot')
    args = parser.parse_args()
    if platform.system() != 'Darwin':
        parser.error('This wrapper requires macOS Seatbelt; no unconfined fallback')
    repo = Path(__file__).resolve().parents[2]
    results = repo / 'evals/promptfoo/results/accuracy-baseline'
    results.mkdir(parents=True, exist_ok=True)
    evidence = {'verifiedAt': datetime.now(timezone.utc).isoformat(),
                'modelRequests': 0, 'dynamicTaskAdmission': False,
                'commands': [], 'canaries': [], 'status': 'failed'}
    lock = None
    process = None
    build_process = None
    runtime = None
    def interrupted(signum, _frame):
        raise KeyboardInterrupt(f'interrupted by signal {signum}')
    for signum in (signal.SIGTERM, signal.SIGINT):
        signal.signal(signum, interrupted)
    try:
        locks = repo / 'target/rust-slot-locks'
        locks.mkdir(parents=True, exist_ok=True)
        while lock is None:
            for number in range(1, 11):
                candidate = locks / f'slot-{number}.lock'
                try:
                    candidate.mkdir()
                except FileExistsError:
                    continue
                lock = candidate
                break
            if lock is None:
                time.sleep(2)
        (lock / 'owner').write_text(json.dumps({'plan': '底层准确度评测', 'task': 19,
                                               'pid': os.getpid()}))
        target = repo / f'target/rust-slot-{number}'
        evidence.update(slot=number, CARGO_TARGET_DIR=str(target),
                        head=run(['git', 'rev-parse', 'HEAD'], cwd=repo).stdout.strip(),
                        sourceStatus=run(['git', 'status', '--porcelain'], cwd=repo).stdout,
                        sourceDiffSha256=hashlib.sha256(subprocess.check_output(
                            ['git', 'diff', 'HEAD', '--', 'crates'], cwd=repo)).hexdigest())
        env = dict(os.environ, CARGO_TARGET_DIR=str(target))
        evidence['rustCargoSourceSha256'] = source_digest(repo)
        if not args.reuse_build:
            command = ['cargo', 'build', '-p', 'vw-cli', '--bin', 'shiju']
            with (results / 'build.log').open('w') as log:
                build_process = subprocess.Popen(command, cwd=repo, env=env, stdout=log,
                                                 stderr=log, start_new_session=True)
                build_process.wait()
            evidence['commands'].append({'argv': command, 'exitCode': build_process.returncode})
            if build_process.returncode:
                raise RuntimeError('default-feature build failed; see ignored build.log')
        if source_digest(repo) != evidence['rustCargoSourceSha256']:
            raise RuntimeError('Rust/Cargo source changed during build; retry with stable sources')
        source_binary = target / 'debug/shiju'
        runtime = Path(tempfile.mkdtemp(prefix='vw-gateway-preflight-', dir='/private/tmp'))
        runtime.chmod(0o700)
        work, state, binary_dir = [runtime / name for name in ('work', 'state', 'bin')]
        for directory in (work, state, binary_dir):
            directory.mkdir(mode=0o700)
        binary = binary_dir / 'shiju'
        # APFS clone avoids copying a large debug executable on every preflight.
        # It remains a distinct inode so the sandbox cannot alter Cargo output.
        cloned = run(['/bin/cp', '-c', str(source_binary), str(binary)])
        if cloned.returncode:
            raise RuntimeError('APFS binary clone failed: ' + cloned.stderr)
        binary.chmod(0o500)
        evidence.update(sourceBinary=str(source_binary), binary=str(binary),
                        binarySha256=digest(binary), runtime=str(runtime), workRoot=str(work),
                        stateRoot=str(state), uid=os.getuid(), platform=platform.platform())
        evidence['workRootIdentity'] = {'realpath': str(work.resolve()),
                                       'dev': work.stat().st_dev, 'ino': work.stat().st_ino}
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0))
            port = sock.getsockname()[1]
        endpoint = f'http://127.0.0.1:{port}'
        evidence['endpoint'] = endpoint
        forbidden = runtime / 'supervisor-only'
        forbidden.mkdir()
        (forbidden / 'sentinel').write_text('disposable-canary\n')
        (work / 'escape').symlink_to(forbidden, target_is_directory=True)
        quote = json.dumps
        # No mach access, daemon IPC, external network, credentials, repository,
        # shared tmp writes or executable modification. Kernel restrictions are
        # inherited across exec/fork regardless of application tool permissions.
        profile = '\n'.join([
            '(version 1)', '(deny default)', '(allow process-exec)',
            '(allow process-fork)',
            '(allow sysctl-read (sysctl-name-regex #"^hw[.]")',
            ' (sysctl-name "kern.osrelease") (sysctl-name "kern.osversion")',
            ' (sysctl-name "kern.ostype") (sysctl-name "kern.version"))',
            '(allow file-read-metadata (literal "/private") (literal "/private/tmp")',
            f' (literal "/usr") (literal "/System") (literal {quote(str(runtime))}))',
            '(allow file-read* (literal "/") (subpath "/System/Library")',
            ' (subpath "/usr/lib") (subpath "/usr/bin") (subpath "/bin")',
            ' (literal "/dev/null") (literal "/dev/random") (literal "/dev/urandom")',
            f' (subpath {quote(str(binary_dir))}) (subpath {quote(str(work))})',
            f' (subpath {quote(str(state))}))',
            f'(allow file-write* (subpath {quote(str(work))}) (subpath {quote(str(state))})',
            ' (literal "/dev/null"))',
            f'(allow network-bind (local ip "localhost:{port}"))',
            f'(allow network-inbound (local ip "localhost:{port}"))',
        ])
        profile_path = results / 'gateway.sb'
        profile_path.write_text(profile)
        evidence['profileSha256'] = digest(profile_path)
        clean_env = {'PATH': '/usr/bin:/bin', 'HOME': str(state), 'TMPDIR': str(state),
                     'VIBEWINDOW_CONFIG_DIR': str(state / 'config'),
                     'CARGO_TARGET_DIR': str(target), 'RUST_LOG': 'warn', 'LANG': 'en_US.UTF-8'}
        evidence['environment'] = clean_env
        prefix = ['/usr/bin/sandbox-exec', '-f', str(profile_path)]
        checks = [
            ('allowed-write', 'printf ok > allowed.txt', True),
            ('allowed-read', 'test "$(cat allowed.txt)" = ok', True),
            ('absolute-read-denied', f'cat {forbidden}/sentinel', False),
            ('absolute-write-denied', f'printf bad > {forbidden}/written', False),
            ('symlink-read-denied', 'cat escape/sentinel', False),
            ('symlink-write-denied', 'printf bad > escape/written', False),
            ('descendant-denied', f'/bin/sh -c "cat {forbidden}/sentinel"', False),
            ('repo-read-denied', f'cat {repo}/Cargo.toml', False),
            ('supervisor-evidence-denied', f'cat {profile_path}', False),
            ('binary-write-denied', f'chmod u+w {binary} && printf bad > {binary}', False),
            ('data-volume-alias-denied', f'cat /System/Volumes/Data{forbidden}/sentinel', False),
            ('new-session-descendant-denied',
             f'/usr/bin/perl -MPOSIX -e \'POSIX::setsid(); exec "/bin/cat", "{forbidden}/sentinel"\'', False),
        ]
        for name, command, allowed in checks:
            result = run(prefix + ['/bin/sh', '-c', command], cwd=work, env=clean_env)
            # A missing command/file or interpreter failure is not a boundary
            # denial. Ignore sh's unrelated optional implementation probe.
            denied = any(line.endswith(': Operation not permitted') and
                         not line.startswith('Error opening /private/var/select/sh:')
                         for line in result.stderr.splitlines())
            passed = result.returncode == 0 if allowed else result.returncode != 0 and denied
            evidence['canaries'].append({'name': name, 'argv': prefix + ['/bin/sh', '-c', command],
                                         'exitCode': result.returncode, 'passed': passed,
                                         'stdout': result.stdout, 'stderr': result.stderr})
            if not passed:
                raise RuntimeError(f'boundary canary failed: {name}')
        # A real supervisor listener distinguishes a sandbox denial from an
        # unavailable destination. Only the Gateway's own listener is allowed.
        with socket.socket() as blocked_listener:
            blocked_listener.bind(('127.0.0.1', 0))
            blocked_listener.listen()
            blocked_port = blocked_listener.getsockname()[1]
            command = ['/usr/bin/curl', '--noproxy', '*', '--max-time', '2',
                       f'http://127.0.0.1:{blocked_port}']
            result = run(prefix + command, cwd=work, env=clean_env)
            blocked_listener.settimeout(0.1)
            try:
                connection, _ = blocked_listener.accept()
                connection.close()
                raise RuntimeError('sandbox connected to supervisor network canary')
            except socket.timeout:
                pass
            passed = result.returncode != 0 and 'Operation not permitted' in result.stderr
            evidence['canaries'].append({'name': 'supervisor-network-denied', 'argv': prefix + command,
                                         'exitCode': result.returncode, 'passed': passed})
            if not passed:
                raise RuntimeError('network denial was not established')
        if (forbidden / 'written').exists() or (forbidden / 'sentinel').read_text() != 'disposable-canary\n':
            raise RuntimeError('forbidden canary changed')
        command = prefix + [str(binary), 'gateway', '--host', '127.0.0.1', '--port', str(port)]
        evidence['launchCommand'] = command
        with (results / 'gateway.log').open('w') as log:
            process = subprocess.Popen(command, cwd=work, env=clean_env,
                                       stdout=log, stderr=log, start_new_session=True)
        evidence.update(pid=process.pid, processStartedAt=datetime.now(timezone.utc).isoformat())
        for attempt in range(1, 61):
            if process.poll() is not None:
                raise RuntimeError(f'Gateway exited {process.returncode}; see ignored gateway.log')
            try:
                with urllib.request.urlopen(endpoint + '/v1/health', timeout=1) as response:
                    evidence['health'] = {'status': response.status,
                                          'body': response.read(16384).decode(), 'attempts': attempt}
                break
            except (OSError, TimeoutError):
                time.sleep(0.5)
        else:
            raise RuntimeError('health timeout')
        # Startup does not require an agent to act. Persist the same local/local
        # tenant used by the tool factory, with read-only application policy in
        # addition to the inherited kernel sandbox. No host config is imported.
        policy = {'autonomy': {'level': 'readonly', 'workspace_only': True,
                              'allowed_roots': [], 'allowed_commands': [],
                              'block_high_risk_commands': True,
                              'require_approval_for_medium_risk': True},
                  'security': {'bash': {'enabled': True}}}
        request = urllib.request.Request(endpoint + '/v1/config', method='PATCH',
            headers={'Content-Type': 'application/json', 'X-Platform-ID': 'local',
                     'X-Company-ID': 'local'}, data=json.dumps(policy).encode())
        try:
            response = urllib.request.urlopen(request, timeout=5)
        except urllib.error.HTTPError as error:
            (results / 'policy-error.json').write_bytes(error.read())
            raise RuntimeError('read-only startup policy rejected; see ignored policy-error.json') from error
        with response:
            if response.status != 200:
                raise RuntimeError('read-only startup policy rejected')
        evidence['policyPatch'] = policy
        for name in ('config', 'tools'):
            request = urllib.request.Request(endpoint + '/v1/' + name,
                                             headers={'X-VibeWindow-Directory': str(work),
                                                      'X-Platform-ID': 'local',
                                                      'X-Company-ID': 'local'})
            try:
                response = urllib.request.urlopen(request, timeout=5)
            except urllib.error.HTTPError as error:
                (results / f'{name}-error.json').write_bytes(error.read())
                raise
            with response:
                payload = response.read(4 * 1024 * 1024)
                (results / f'{name}.json').write_bytes(payload)
                evidence[name + 'Sha256'] = hashlib.sha256(payload).hexdigest()
                data = json.loads(payload)
                if name == 'config':
                    evidence['modelConfiguration'] = {key: data.get(key) for key in
                        ('default_provider', 'default_model', 'workspace_dir', 'autonomy')}
                    if data['autonomy']['level'] != 'readonly' or not data['autonomy']['workspace_only']:
                        raise RuntimeError('effective application policy does not match startup policy')
                else:
                    evidence['toolIds'] = [item.get('name', item.get('id'))
                                           for item in data.get('items', [])]
        evidence['modelAvailability'] = 'not-tested; no credentials or outbound network'
        evidence['forbiddenWriteScopes'] = [str(repo), str(Path.home()), str(forbidden), str(results)]
        evidence['configurationDigest'] = hashlib.sha256(json.dumps({key: evidence[key] for key in
            ('profileSha256', 'binarySha256', 'environment', 'configSha256', 'toolsSha256',
             'workRoot', 'stateRoot', 'uid', 'platform')}, sort_keys=True).encode()).hexdigest()
        evidence['startupAdmission'] = {
            'mode': 'restricted-sandbox', 'scope': 'credential-free-startup-only',
            'attemptId': f'gateway-preflight-{process.pid}',
            'configurationDigest': evidence['configurationDigest'], 'workRoot': str(work),
            'allowedWriteScopes': [str(work), str(state)],
            'forbiddenWriteScopes': evidence['forbiddenWriteScopes'],
            'evidencePath': str(results / 'environment.json'),
            'verifiedAt': evidence['verifiedAt'], 'validCanaries': len(checks) + 1,
            'externalNetwork': 'denied', 'machIPC': 'denied', 'hostMounts': [],
            'modelCredentials': 'none', 'validForLaterAttempts': False,
        }
        evidence['startupAdmission']['digest'] = hashlib.sha256(json.dumps(
            evidence['startupAdmission'], sort_keys=True).encode()).hexdigest()
        evidence['status'] = 'startup-and-boundary-canaries-passed'
    except (Exception, KeyboardInterrupt) as error:
        evidence['error'] = str(error)
    finally:
        if build_process is not None:
            try:
                os.killpg(build_process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            build_process.wait()
        if process is not None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=5)
            evidence['gatewayExitCode'] = process.returncode
            try:
                os.killpg(process.pid, 0)
                evidence['processGroupGone'] = False
            except ProcessLookupError:
                evidence['processGroupGone'] = True
            if not evidence['processGroupGone']:
                evidence['status'] = 'failed'
                evidence['error'] = 'Gateway process group remains; slot retained'
                lock = None
            with socket.socket() as probe:
                evidence['endpointClosed'] = probe.connect_ex(('127.0.0.1', port)) != 0
            if not evidence['endpointClosed']:
                evidence['status'] = 'failed'
                evidence['error'] = 'Gateway endpoint remains open'
        if lock is not None:
            (lock / 'owner').unlink(missing_ok=True)
            lock.rmdir()
            evidence['slotReleased'] = True
        if runtime is not None and (process is None or evidence.get('processGroupGone')):
            shutil.rmtree(runtime)
            evidence['runtimeRemoved'] = True
        evidence['validCanaries'] = sum(c['passed'] for c in evidence['canaries'])
        (results / 'environment.json').write_text(json.dumps(evidence, indent=2, ensure_ascii=False) + '\n')
        print(json.dumps({key: evidence.get(key) for key in
                          ('status', 'error', 'endpoint', 'slot', 'CARGO_TARGET_DIR', 'validCanaries')}))
    return 0 if evidence['status'] == 'startup-and-boundary-canaries-passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
