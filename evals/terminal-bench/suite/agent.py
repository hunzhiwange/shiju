"""Harbor suite adapter: explicit frozen manifest, private Qwen profile.

Select suite.agent:SuiteAgent; pass runtime_manifest, task_id, runtime_dir and
profile_path. Manifest uses model {id, max_output_tokens, input_window_tokens},
transport tasks {taskId, workingDirectory, agentTimeoutSec}, and runtime
{binarySha256, requiredSidecars}, without falling back to the legacy manifest.
Official task content/scoring stays in Harbor's unchanged task package.
"""
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import shlex
import tempfile
import uuid

from harbor.agents.installed.base import BaseInstalledAgent
from vibewindow_agent import VibeWindowAgent

MODEL = 'token-plan/qwen3.8-flash'
LIMIT = {'context': 200000, 'input': 200000, 'output': 32000}
_BASE = Path(__file__).resolve().parent.parent


def source_identity():
    """Bind the wrapper, inherited transport and immutable HTTP bootstrap."""
    files = {name: hashlib.sha256((_BASE / name).read_bytes()).hexdigest()
             for name in ('suite/agent.py', 'vibewindow_agent.py', 'bind-profile.sh')}
    digest = hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()
    return {'sha256': digest, 'files': files}


def _load(path):
    try:
        value = json.loads(Path(path).read_text())
        if not isinstance(value, dict):
            raise ValueError('expected object')
        return value
    except (OSError, TypeError, ValueError):
        raise ValueError('invalid suite configuration') from None


def _validate_profile(path):
    if path is None or not Path(path).is_file() or Path(path).stat().st_mode & 0o777 != 0o600:
        raise ValueError('suite requires a temporary 0600 profile')
    profile = _load(path)
    provider, auth = profile.get('provider', {}), profile.get('auth', {})
    if (set(profile) != {'provider', 'auth'}
            or not isinstance(provider, dict) or not isinstance(auth, dict)
            or set(provider) != {'api', 'adapter', 'models'}
            or not isinstance(provider['api'], str) or not provider['api']
            or provider['adapter'] != 'openai-compatible'
            or provider['models'] != {'qwen3.8-flash': {'status': 'active', 'limit': LIMIT}}
            or any(type(provider['models']['qwen3.8-flash']['limit'][key]) is not int for key in LIMIT)
            or set(auth) != {'type', 'key'} or auth['type'] != 'api'
            or not isinstance(auth['key'], str) or not auth['key']):
        raise ValueError('suite profile differs from frozen model, budget or API auth')
    return profile


class SuiteAgent(VibeWindowAgent):
    def __init__(self, logs_dir, *, task_id, runtime_manifest,
                 profile_path=None, runtime_dir=None, model_name=None, **kwargs):
        _validate_profile(profile_path)
        manifest = _load(runtime_manifest)
        expected = {'id': MODEL, 'max_output_tokens': 32000, 'input_window_tokens': 200000}
        if (manifest.get('model') != expected or model_name not in (None, MODEL)
                or any(type(manifest['model'][key]) is not int
                       for key in ('max_output_tokens', 'input_window_tokens'))):
            raise ValueError('suite manifest differs from frozen model or budget')
        tasks = manifest.get('tasks')
        if not isinstance(tasks, list) or any(not isinstance(task, dict) for task in tasks):
            raise ValueError('suite manifest requires explicit tasks')
        selected = [task for task in tasks if task.get('taskId') == task_id]
        if len(selected) != 1:
            raise ValueError('task_id must identify one suite manifest task')
        task = selected[0]
        directory, timeout = task.get('workingDirectory'), task.get('agentTimeoutSec')
        if (not isinstance(directory, str) or not PurePosixPath(directory).is_absolute()
                or isinstance(timeout, bool) or not isinstance(timeout, (int, float))
                or not math.isfinite(timeout) or timeout <= 0):
            raise ValueError('suite task requires absolute working directory and original timeout')
        runtime = manifest.get('runtime', {})
        if not isinstance(runtime, dict) or not isinstance(runtime.get('requiredSidecars'), dict):
            raise ValueError('suite manifest requires runtime identity')
        digests = {'shiju': runtime.get('binarySha256'), **runtime['requiredSidecars']}
        if 'libsqlite3.so' not in digests or any(
                PurePosixPath(name).name != name or name in ('.', '..')
                or not isinstance(digest, str) or len(digest) != 64
                or any(char not in '0123456789abcdef' for char in digest)
                for name, digest in digests.items()):
            raise ValueError('suite runtime requires safe sidecar names and SHA-256 digests')
        # Call Harbor directly: the legacy initializer reads a different manifest.
        BaseInstalledAgent.__init__(self, logs_dir=Path(logs_dir), model_name=MODEL, **kwargs)
        self.task, self.manifest = task, manifest
        self.runtime_dir = Path(runtime_dir) if runtime_dir else None
        self.profile_path = Path(profile_path)
        self.remote = '/tmp/vw-harbor-' + uuid.uuid4().hex
        self.config_directory = self.remote + '/config'
        self.runtime_env = {
            'HOME': self.remote + '/home', 'RUST_LOG': 'warn',
            'LD_LIBRARY_PATH': self.remote + '/runtime',
            'SSL_CERT_FILE': self.remote + '/runtime/ca-certificates.crt',
            'OPENSSL_CONF': self.remote + '/runtime/openssl.cnf',
        }
        self.suite_source_identity = source_identity()
        self.manifest_sha256 = hashlib.sha256(Path(runtime_manifest).read_bytes()).hexdigest()
        self._attempted = False

    @staticmethod
    def name():
        return 'vibewindow-suite'

    def version(self):
        return 'qwen38-suite-' + self.suite_source_identity['sha256']

    async def _bind_profile(self, environment, remote):
        profile = _validate_profile(self.profile_path)
        with tempfile.TemporaryDirectory() as temp:
            for name, payload in (('provider.json', {'providers': {'token-plan': profile['provider']}}),
                                  ('auth.json', profile['auth'])):
                path = Path(temp) / name
                # Create privately before writing any credential bytes.
                with open(path, 'x', opener=lambda name, flags: os.open(name, flags, 0o600)) as file:
                    json.dump(payload, file, ensure_ascii=True)
                await environment.upload_file(path, remote + '/' + name)
            await environment.upload_file(_BASE / 'bind-profile.sh', remote + '/bind-profile.sh')
        bound = await self._execute(environment, 'bash ' + shlex.quote(remote + '/bind-profile.sh')
                                    + ' ' + shlex.quote(remote))
        if bound.return_code:
            raise RuntimeError('suite container profile binding failed')
        return True

    async def run(self, instruction, environment, context):
        if self._attempted:
            raise ValueError('suite paid retry forbidden')
        self._attempted = True
        try:
            await super().run(instruction, environment, context)
        finally:
            result = (context.metadata or {}).get('vibewindow')
            if result is not None:
                result['base_adapter_source_sha256'] = result['adapter_source_sha256']
                result['adapter_source_sha256'] = self.suite_source_identity['sha256']
                result.update(suite_source_identity=self.suite_source_identity,
                              suite_manifest_sha256=self.manifest_sha256,
                              max_output_tokens=32000, input_window_tokens=200000,
                              output_truncated=result.get('finish_reason') == 'length')
                (Path(self.logs_dir) / result['request_id'] / 'result.json').write_text(
                    json.dumps(result, indent=2))
