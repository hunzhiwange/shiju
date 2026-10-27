"""Durable attempt state and execution identity; unknown never grants a retry."""
from contextlib import contextmanager
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import uuid

BASE = Path(__file__).resolve().parents[1]
GLOBAL_LOCK = BASE.parents[1] / 'target/qwen38-suite-model.lock'
MODEL = {'id': 'token-plan/qwen3.8-flash', 'max_output_tokens': 32000,
         'input_window_tokens': 200000}

def load(path):
    return json.loads(Path(path).read_text())


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def atomic_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(dir=path.parent, prefix='.atomic-')
    try:
        with os.fdopen(descriptor, 'w') as stream:
            json.dump(value, stream, sort_keys=True, allow_nan=False)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        directory = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        Path(temporary).unlink(missing_ok=True)


@contextmanager
def locked(path, blocking=False):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('a+') as stream:
        fcntl.flock(stream, fcntl.LOCK_EX | (0 if blocking else fcntl.LOCK_NB))
        yield


def process_identity(pid):
    result = subprocess.run(['ps', '-p', str(pid), '-o', 'lstart=', '-o', 'command='],
                            capture_output=True, text=True, check=False)
    return result.stdout.strip() if result.returncode == 0 else None


def reserve(output, case_id, manifest_sha256):
    if not re.fullmatch('[a-z0-9][a-z0-9-]*', case_id):
        raise ValueError('unsafe case ID')
    attempt = Path(output) / 'attempts' / case_id
    attempt.mkdir(parents=True, exist_ok=False)
    atomic_json(attempt / 'reservation.json', {
        'attempt_id': uuid.uuid4().hex, 'case_id': case_id,
        'manifest_sha256': manifest_sha256, 'attempt_number': 1,
        'started_at': datetime.now(timezone.utc).isoformat()})
    return attempt


def admit(output, case_id, manifest):
    """Bind an experiment to one output, and retain an orphan request barrier."""
    with locked(GLOBAL_LOCK.with_suffix('.admission.lock')):
        registry = GLOBAL_LOCK.parent / 'qwen38-suite-experiments'
        registry.mkdir(exist_ok=True)
        identity = manifest['experiment_id']
        if not re.fullmatch('[a-zA-Z0-9-]+', identity):
            raise ValueError('unsafe experiment identity')
        binding = registry / (identity + '.json')
        expected = {'output': str(Path(output).resolve()), 'manifest_sha256': manifest['manifest_sha256']}
        if binding.exists() and load(binding) != expected:
            raise ValueError('experiment already bound to another output or manifest')
        active = GLOBAL_LOCK.with_suffix('.active.json')
        if active.exists() and reconcile(load(active)['attempt'])['state'] != 'finished':
            raise RuntimeError('previous model attempt still running or unknown; no new request permitted')
        atomic_json(binding, expected)
        attempt = reserve(output, case_id, manifest['manifest_sha256'])
        atomic_json(active, {'attempt': str(attempt)})
        return attempt


def reconcile(attempt):
    attempt = Path(attempt)
    if (attempt / 'terminal.json').exists():
        terminal = load(attempt / 'terminal.json')
        reservation = load(attempt / 'reservation.json')
        if any(terminal.get(key) != reservation[key] for key in ('attempt_id', 'case_id')):
            raise ValueError('terminal attempt identity mismatch')
        return terminal
    if (attempt / 'owner.json').exists():
        owner = load(attempt / 'owner.json')
        identity = process_identity(owner['pid'])
        # PID reuse is insufficient: require exact start time AND command plus
        # the worker's held lease. A reused PID cannot inherit the lease.
        if identity and identity == owner['start']:
            try:
                with locked(attempt / 'worker.lock'):
                    pass
            except BlockingIOError:
                return {'state': 'running', 'reward': None}
    return {'state': 'unknown', 'score_status': 'unscored', 'reward': None}


def tree(path):
    path = Path(path)
    result = {}
    for file in sorted(path.rglob('*')):
        if file.is_symlink():
            raise ValueError('symlink in frozen task')
        if file.is_file():
            result[file.relative_to(path).as_posix()] = {
                'sha256': digest(file), 'executable': bool(file.stat().st_mode & 0o111)}
    return result



def execution_identity():
    names = ('run.py', 'state.py', 'scoring.py', 'preparation.py', 'budget.py',
             'budget_gate.rs', 'contract.py', 'schema.json', 'runtime.py', 'runtime-v2.json')
    files = {name: digest(Path(__file__).with_name(name)) for name in names}
    return hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()
