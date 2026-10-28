"""Official Harbor scoring, verifier-only recovery and contract evidence."""
import asyncio
from contextlib import nullcontext
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import re
import shlex
import shutil
import time
import tempfile
import uuid

from harbor.verifier.verifier import Verifier
from suite.state import MODEL, atomic_json, tree


async def bootstrap_verifier(environment, policy, archive):
    """Inject hash-pinned dependency caches after the agent, never into /app."""
    if set(policy) != {'files', 'script', 'env'} or not policy['files']:
        raise ValueError('invalid verifier bootstrap')
    allowed_env = {'PATH', 'LD_LIBRARY_PATH', 'UV_DOWNLOAD_URL', 'UV_PYTHON_DOWNLOADS',
                   'UV_FIND_LINKS', 'UV_OFFLINE'}
    if not isinstance(policy['env'], dict) or set(policy['env']) - allowed_env:
        raise ValueError('unsupported verifier bootstrap environment')
    prefix = '/opt/vw-verifier-deps/'
    files = []
    for item in policy['files']:
        if set(item) != {'source', 'target', 'sha256'}:
            raise ValueError('invalid verifier dependency file')
        target = item['target']
        if not isinstance(target, str) or not re.fullmatch(r'[A-Za-z0-9_.-]+', target) or target in ('.', '..'):
            raise ValueError('unsafe verifier dependency target')
        source = Path(item['source'])
        if (not source.is_absolute() or source.is_symlink() or not source.is_file()
                or hashlib.sha256(source.read_bytes()).hexdigest() != item['sha256']):
            raise ValueError('verifier dependency missing or changed')
        files.append((source, prefix + target, item['sha256']))
    targets = [remote for _, remote, _ in files]
    if len(set(targets)) != len(targets) or prefix + policy['script'] not in targets:
        raise ValueError('duplicate dependency or missing bootstrap script')
    if any(not isinstance(value, str) or '\0' in value for value in policy['env'].values()):
        raise ValueError('invalid verifier dependency environment')
    created = await environment.exec(command='mkdir -p /opt/vw-verifier-deps')
    if created.return_code != 0:
        raise ValueError('cannot create verifier dependency directory')
    for source, remote, _ in files:
        await environment.upload_file(source, remote)
    checks = '\n'.join(f'{digest}  {remote}' for _, remote, digest in files) + '\n'
    command = ('printf %s ' + shlex.quote(checks) + ' | sha256sum -c - && bash '
               + shlex.quote(prefix + policy['script']))
    result = await environment.exec(command=command, env=policy['env'])
    atomic_json(Path(archive) / 'bootstrap.json',
                {'policy': policy, 'exit_code': result.return_code,
                 'stdout': result.stdout, 'stderr': result.stderr, 'phase': 'verifier-only'})
    if result.return_code != 0:
        raise ValueError('verifier dependency bootstrap failed')
    return policy['env'].copy()


class RecoveringVerifier(Verifier):
    """One verifier-only recovery; original results/logs are never overwritten."""
    def __init__(self, *args, recovery=None, **kwargs):
        super().__init__(*args, **kwargs)
        self.recovery = recovery or {'mode': 'service'}

    async def verify(self):
        from harbor.models.trial.paths import TrialPaths
        from suite.docker import SuiteDockerEnvironment
        started = time.monotonic()
        limit = self.task.config.verifier.timeout_sec
        archive = self.trial_paths.trial_dir / 'scoring'
        archive.mkdir(exist_ok=False)
        snapshot = archive / 'artifact'
        snapshot.mkdir()
        policy = self.recovery
        if policy.get('bootstrap'):
            if policy['mode'] != 'service':
                raise ValueError('bootstrap requires original-environment verification')
            env = await asyncio.wait_for(
                bootstrap_verifier(self.environment, policy['bootstrap'], archive), limit)
            self.override_env = {**self.override_env, **env}
        if policy['mode'] == 'artifact':
            for remote in policy['paths']:
                if not remote.startswith('/') or '..' in Path(remote).parts:
                    raise ValueError('unsafe artifact path')
                local = snapshot / remote.lstrip('/')
                local.parent.mkdir(parents=True, exist_ok=True)
                exists = await self.environment.exec(command='test -f ' + shlex.quote(remote))
                if exists.return_code == 0:
                    await self.environment.download_file(remote, local)
                elif exists.return_code != 1:
                    raise ValueError('artifact probe failed')
        fingerprint = hashlib.sha256(json.dumps(tree(snapshot), sort_keys=True).encode()).hexdigest()
        failure = None
        try:
            first = await asyncio.wait_for(super().verify(), max(0.001, limit - (time.monotonic() - started)))
            atomic_json(archive / 'first.json', first.model_dump(mode='json'))
        except Exception as error:
            failure = error
            atomic_json(archive / 'first.json', {'error_type': type(error).__name__})
        if failure is None and not policy.get('calibrate'):
            return first
        shutil.copytree(self.trial_paths.verifier_dir, archive / 'first-logs', dirs_exist_ok=True)
        receipt = {'mode': policy['mode'], 'elapsed': time.monotonic() - started,
                   'verifier_limit': limit, 'artifact_sha256': fingerprint,
                   'original_environment': True, 'calibrated': policy.get('calibrated', False)}
        atomic_json(archive / 'receipt.json', receipt)
        remaining = recovery_budget(receipt, fingerprint)
        paths = TrialPaths(trial_dir=archive / 'recovery')
        paths.mkdir()
        environment = self.environment
        separate = policy['mode'] == 'artifact'
        if separate:
            image = policy['image']
            if not re.fullmatch(r'[a-zA-Z0-9_./:-]+@sha256:[a-f0-9]{64}', image):
                raise ValueError('recovery image must be digest pinned')
            environment = SuiteDockerEnvironment(
                environment_dir=self.task.paths.environment_dir, environment_name='verifier-recovery',
                session_id=uuid.uuid4().hex, trial_paths=paths,
                task_env_config=self.task.config.environment.model_copy(update={'docker_image': image}))
        try:
            async def grade():
                if separate:
                    await environment.start(force_build=False)
                    for local in snapshot.rglob('*'):
                        if local.is_file():
                            await environment.upload_file(local, '/' + local.relative_to(snapshot).as_posix())
                verifier = Verifier(task=self.task, trial_paths=paths, environment=environment,
                                    override_env=self.override_env, verifier_env=self.verifier_env)
                return await verifier.verify()
            recovered = await asyncio.wait_for(grade(), remaining)
            if hashlib.sha256(json.dumps(tree(snapshot), sort_keys=True).encode()).hexdigest() != fingerprint:
                raise ValueError('artifact changed')
            atomic_json(archive / 'recovered.json', recovered.model_dump(mode='json'))
            receipt['elapsed'] = time.monotonic() - started
            atomic_json(archive / 'receipt.json', receipt)
            return recovered
        finally:
            if separate:
                # The image's idle PID 1 can ignore TERM. Kill only this unique
                # recovery compose project before removal, avoiding a 10s grace
                # delay inside the original scoring budget.
                await environment._run_docker_compose_command(['kill'], check=False)
                await environment.stop(delete=True)


def summarize(result):
    metadata = ((result.get('agent_result') or {}).get('metadata') or {})
    transport = metadata.get('vibewindow', {})
    rewards = (result.get('verifier_result') or {}).get('rewards') or {}
    reward = rewards.get('reward')
    valid = (not result.get('exception_info') and transport.get('status', 'completed') == 'completed'
             and type(reward) in (int, float)
             and math.isfinite(reward) and 0 <= reward <= 1)
    return {'state': 'finished', 'transport_status': transport.get('status', 'unknown'),
            'terminal_event': transport.get('terminal_event', (transport.get('terminal') or {}).get('type')),
            'score_status': 'scored' if valid else 'unscored',
            'reward': reward if valid else None,
            'failure_class': ('none' if reward == 1 else 'capability') if valid else 'infrastructure'}


def experiment_record(manifest, task, reservation, result, profile_sha):
    from suite.contract import validate_record
    transport = (((result.get('agent_result') or {}).get('metadata') or {}).get('vibewindow') or {})
    transport = {**transport, 'raw_exit_code': transport.get('exit_code'),
                 'terminal_event': (transport.get('terminal') or {}).get('type'),
                 'stream_terminated': transport.get('stream_terminated', False),
                 'error_seen': transport.get('error_seen', False)}
    summary = summarize(result)
    normal = (transport.get('status') == 'completed' and transport.get('raw_exit_code') == 0
              and transport.get('terminal_event') == 'chat.done'
              and transport.get('finish_reason') in ('stop', 'end_turn')
              and transport.get('stream_terminated') is True and not transport.get('error_seen')
              and not transport.get('output_truncated') and summary['score_status'] == 'scored')
    def hash_files(prefix):
        files = {name: value for name, value in task['files'].items() if name.startswith(prefix)}
        return hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()
    timing = result.get('verifier') or {}
    elapsed = 0
    if timing.get('started_at') and timing.get('finished_at'):
        elapsed = (datetime.fromisoformat(timing['finished_at']) -
                   datetime.fromisoformat(timing['started_at'])).total_seconds()
    usage = transport.get('usage')
    if isinstance(usage, dict) and {'input_tokens', 'output_tokens'} <= usage.keys():
        usage = {k: v for k, v in usage.items() if k in
                 ('input_tokens', 'output_tokens', 'cached_tokens', 'reasoning_tokens')}
    else:
        usage = None
    row = {**MODEL, 'schema_version': 1, 'experiment_id': manifest['experiment_id'],
           'case_id': task['taskId'], 'kind': task['kind'], 'attempt_number': 1,
           'attempt_id': reservation['attempt_id'], 'model': MODEL['id'],
           'hashes': {'source': manifest['adapter']['sha256'],
                      'runtime': manifest['runtime']['binarySha256'], 'fixture': hash_files('environment/'),
                      'checker': hash_files('tests/'), 'profile': profile_sha},
           'started_at': reservation['started_at'], 'ended_at': datetime.now(timezone.utc).isoformat(),
           'validation_seconds': max(0, elapsed), 'usage_status': 'reported' if usage else 'missing',
           'usage': usage if usage else None, 'raw_exit_code': transport.get('raw_exit_code'),
           'finish_reason': transport.get('finish_reason'), 'terminal_event': transport.get('terminal_event'),
           'stream_terminated': transport.get('stream_terminated', False),
           'error_seen': transport.get('error_seen', False),
           'output_truncated': transport.get('output_truncated', False),
           'status': 'completed' if normal else 'failed',
           'score_status': 'scored' if normal else 'unscored',
           'reward': summary['reward'] if normal else None,
           'failure_class': summary['failure_class'] if normal else 'infrastructure'}
    if 'environment_variant' in task:
        row['environment_variant'] = task['environment_variant']
    row.pop('id')
    validate_record(row)
    return row


def recovery_budget(receipt, artifact_sha256):
    remaining = receipt['verifier_limit'] - receipt['elapsed']
    if not math.isfinite(remaining) or remaining <= 0:
        raise ValueError('original verifier budget exhausted')
    if receipt['mode'] == 'artifact':
        if not receipt.get('calibrated') or receipt.get('artifact_sha256') != artifact_sha256:
            raise ValueError('recovery requires identical artifact and calibrated verifier image')
    elif receipt['mode'] != 'service' or not receipt.get('original_environment'):
        raise ValueError('service scoring requires original environment')
    return remaining


CASES = Path(__file__).with_name('cases')
IMAGE_CMD_OVERRIDE = b'services:\n  main:\n    command: null\n'


def checked_startup_override(task_id, descriptor):
    """Restore only the image CMD, from trusted case bytes; never arbitrary Compose."""
    if (not isinstance(task_id, str)
            or not re.fullmatch(r'[a-z0-9][a-z0-9-]*(?:\.[a-z0-9][a-z0-9-]*)*', task_id)):
        raise ValueError('unsafe startup override task ID')
    if not isinstance(descriptor, dict) or set(descriptor) != {'source', 'sha256'}:
        raise ValueError('invalid startup override descriptor')
    name, expected = descriptor['source'], descriptor['sha256']
    if (not isinstance(name, str) or not re.fullmatch(r'[a-z0-9][a-z0-9_-]*\.yaml', name)
            or not isinstance(expected, str) or not re.fullmatch(r'[a-f0-9]{64}', expected)):
        raise ValueError('unsafe startup override source or hash')
    case = CASES / task_id
    environment = case / 'environment'
    if any(path.is_symlink() or not path.is_dir() for path in (CASES, case, environment)):
        raise ValueError('untrusted startup override environment')
    path = environment / name
    if path.is_symlink() or not path.is_file() or path.resolve().parent != environment.resolve():
        raise ValueError('missing or external startup override')
    body = path.read_bytes()
    if hashlib.sha256(body).hexdigest() != expected:
        raise ValueError('startup override hash drift')
    if body != IMAGE_CMD_OVERRIDE:
        raise ValueError('startup override must contain only canonical main.command: null')
    return path


async def harbor_trial(task_path, directory, name, agent, kwargs=None, hook=None, recovery=None,
                       startup_override=None, environment_variant=None):
    from harbor.models.trial.config import TrialConfig
    from harbor.trial.trial import Trial
    from harbor.trial.hooks import TrialEvent
    if environment_variant is not None:
        from suite.variants import check_package
        environment_variant = check_package(task_path, environment_variant)
    source = (checked_startup_override(Path(task_path).name, startup_override)
              if startup_override is not None else None)
    # Harbor reads an immutable owned snapshot, never a mutable caller path.
    context = tempfile.TemporaryDirectory(prefix='vw-image-cmd-') if source else nullcontext()
    with context as staging:
        environment = {'import_path': 'suite.docker:SuiteDockerEnvironment', 'delete': False,
                       'cpu_enforcement_policy': 'limit', 'memory_enforcement_policy': 'limit'}
        if environment_variant is not None:
            environment['kwargs'] = {'environment_variant': environment_variant}
        if source:
            snapshot = Path(staging) / source.name
            snapshot.write_bytes(IMAGE_CMD_OVERRIDE)
            snapshot.chmod(0o400)
            environment['extra_docker_compose'] = [str(snapshot)]
        config = TrialConfig.model_validate({
            'task': {'path': str(task_path)}, 'trial_name': name, 'trials_dir': str(directory),
            'agent': ({'name': agent} if agent in ('oracle', 'nop') else
                      {'import_path': agent, 'model_name': MODEL['id'], 'kwargs': kwargs}),
            'verifier': {'import_path': 'suite.scoring:RecoveringVerifier',
                         'kwargs': {'recovery': recovery or {'mode': 'service'}}},
            'environment': environment})
        trial = await Trial.create(config)
        if source:
            async def check_override(event):
                checked_startup_override(Path(task_path).name, startup_override)
                if snapshot.read_bytes() != IMAGE_CMD_OVERRIDE:
                    raise ValueError('startup override snapshot drift')
            trial.add_hook(TrialEvent.ENVIRONMENT_START, check_override)
            trial.add_hook(TrialEvent.AGENT_START, check_override)
        if hook:
            for event in TrialEvent:
                trial.add_hook(event, hook)
        return (await trial.run()).model_dump(mode='json')


def scrub(directory, secret):
    """Preserve all evidence, replacing only exact credential bytes in text logs."""
    if not secret:
        return
    for path in Path(directory).rglob('*'):
        if path.is_file() and not path.is_symlink():
            data = path.read_bytes()
            if secret.encode() in data:
                path.write_bytes(data.replace(secret.encode(), b'[REDACTED]'))
