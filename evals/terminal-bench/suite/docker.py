"""Create-time Docker limits shared by suite trials, calibration and recovery.

Harbor's runtime resource policy does not constrain Compose image builds. Keep
its resolved contexts/tags and standard start sequence, but use the classic
builder's supported cgroup limits and forbid implicit builds during up.
"""
import asyncio
from contextlib import asynccontextmanager
import json
import math
import os
from pathlib import Path
import re
import shlex

from harbor.environments.base import ExecResult
from harbor.environments.docker.docker import DockerEnvironment
from harbor.environments.docker.utils import (default_docker_platform, docker_build_context_hash,
                                              docker_image_exists, _compute_image_name)
from suite.state import locked, atomic_json

LOCK = Path(__file__).resolve().parents[3] / 'target/suite-docker-resources.lock'
MAX_CPU = 4
MAX_MEMORY = 8192 * 2**20
SIDECAR = 'harbor-docker-egress-control-sidecar'


def memory_bytes(value):
    match = re.fullmatch(r'([0-9]+)([bkmg]?)', str(value).lower())
    if not match:
        raise ValueError('unsupported Docker memory limit')
    return int(match[1]) * {'': 1, 'b': 1, 'k': 2**10, 'm': 2**20, 'g': 2**30}[match[2]]


def limits(cpu, memory):
    try:
        cpu, memory = float(cpu), memory_bytes(memory)
    except (TypeError, ValueError):
        raise ValueError('missing suite Docker resource limits') from None
    if not math.isfinite(cpu) or not 0.00001 <= cpu <= MAX_CPU or not 0 < memory <= MAX_MEMORY:
        raise ValueError('missing or excessive suite Docker resource limits')
    return cpu, memory


def build_command(service, cpu, memory):
    cpu, memory = limits(cpu, memory)
    build = service['build']
    if not isinstance(build, dict) or set(build) - {'context', 'dockerfile', 'args', 'target', 'network', 'cache_from', 'labels'}:
        raise ValueError('unsupported constrained Compose build; refusing unlimited fallback')
    context = Path(build['context'])
    if not context.is_absolute() or not context.is_dir():
        raise ValueError('build context must be an existing local absolute directory')
    dockerfile = Path(build.get('dockerfile', 'Dockerfile'))
    if not dockerfile.is_absolute():
        dockerfile = context / dockerfile
    command = ['docker', 'build', '--memory', str(memory), '--memory-swap', str(memory),
               '--cpu-period', '100000', '--cpu-quota', str(math.floor(cpu * 100000)),
               '-t', service['image'], '-f', str(dockerfile)]
    if service.get('platform'):
        command += ['--platform', service['platform']]
    for key, flag in (('target', '--target'), ('network', '--network')):
        if key in build:
            command += [flag, build[key]]
    for key, flag in (('args', '--build-arg'), ('labels', '--label')):
        for name, value in (build.get(key) or {}).items():
            command += [flag, name if value is None else f'{name}={value}']
    for image in build.get('cache_from', []):
        command += ['--cache-from', image]
    return command + [str(context)]


def active_resources(containers):
    cpu, memory = 0, 0
    for container in containers:
        # The existing business stack has a separate reserved host budget.
        if container['Name'].startswith('/dhb168-local-docker-'):
            continue
        host = container['HostConfig']
        quota = host.get('CpuQuota', 0)
        period = host.get('CpuPeriod', 0)
        used_cpu = host.get('NanoCpus', 0) / 1e9
        if not used_cpu and quota > 0 and period > 0:
            used_cpu = quota / period
        used_cpu, used_memory = limits(used_cpu, host.get('Memory', 0))
        cpu += used_cpu
        memory += used_memory
    return cpu, memory


class SuiteDockerEnvironment(DockerEnvironment):
    def __init__(self, *args, environment_variant=None, **kwargs):
        super().__init__(*args, **kwargs)
        self.environment_variant = environment_variant

    async def start(self, *args, **kwargs):
        from suite import variants
        descriptor = self.environment_variant
        if descriptor is not None:
            descriptor = variants.resolve_descriptor(descriptor)
            if self.task_env_config.docker_image != descriptor['image']:
                raise ValueError('variant image must be selected before agent execution')
        await super().start(*args, **kwargs)
        if descriptor is not None:
            result = await self.exec(command=variants.probe_command(descriptor))
            if result.return_code != 0:
                raise ValueError('variant dependency verification failed before agent')
            receipt = {'environment_variant': descriptor, 'phase': 'before_agent',
                       'facts': json.loads(result.stdout)}
            variants.check_start(receipt, descriptor)
            atomic_json(self.trial_paths.trial_dir / 'environment-variant.json', receipt)

    @property
    def _docker_compose_paths(self):
        paths = super()._docker_compose_paths
        overlay = getattr(self, '_suite_resource_overlay', None)
        return paths + ([overlay] if overlay else [])

    async def _docker(self, args):
        process = await asyncio.create_subprocess_exec(
            'docker', *args, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE)
        try:
            stdout, stderr = await process.communicate()
        except BaseException:
            if process.returncode is None:
                process.kill()
            await process.wait()
            raise
        if process.returncode:
            raise RuntimeError('Docker resource query failed: ' + stderr.decode(errors='replace'))
        return stdout.decode()

    @asynccontextmanager
    async def _resource_lock(self):
        # Waiting must remain cancellable by Harbor's existing trial timeout.
        # A blocking flock here would also stop that timeout's event loop.
        while True:
            lock = locked(LOCK)
            try:
                lock.__enter__()
            except BlockingIOError:
                await asyncio.sleep(0.1)
                continue
            break
        try:
            yield
        finally:
            lock.__exit__(None, None, None)

    async def _admit(self, cpu, memory):
        ids = (await self._docker(['ps', '-q'])).split()
        running = json.loads(await self._docker(['inspect', *ids])) if ids else []
        used_cpu, used_memory = active_resources(running)
        if used_cpu + cpu > MAX_CPU or used_memory + memory > MAX_MEMORY:
            raise ValueError('suite Docker capacity exceeds 4 CPU / 8 GiB')
        receipt = {'active_cpu': used_cpu, 'active_memory_bytes': used_memory,
                   'reserved_cpu': cpu, 'reserved_memory_bytes': memory}
        atomic_json(self.trial_paths.trial_dir / 'resource-admission.json', receipt)
        with (self.trial_paths.trial_dir / 'resource-admissions.jsonl').open('a') as history:
            history.write(json.dumps(receipt, sort_keys=True) + '\n')
            history.flush()
            os.fsync(history.fileno())

    async def _limited_build(self, service, cpu, memory):
        command = build_command(service, cpu, memory)
        await self._admit(cpu, memory)
        # BuildKit accepts/ignores some legacy resource flags. The classic
        # builder creates every RUN container with these cgroups from birth.
        environment = self._compose_env_vars(include_os_env=True)
        environment['DOCKER_BUILDKIT'] = '0'
        process = await asyncio.create_subprocess_exec(
            *command, env=environment, cwd=self.environment_dir,
            stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.STDOUT)
        try:
            output, _ = await process.communicate()
        except BaseException:
            if process.returncode is None:
                process.terminate()
            await process.wait()
            # No automatic restart; an interrupted builder requires ownership
            # investigation, just like an interrupted model attempt.
            raise
        log = self.trial_paths.trial_dir / 'constrained-build.log'
        with log.open('ab') as stream:
            stream.write(output)
        if process.returncode:
            raise RuntimeError('constrained Docker build failed; see constrained-build.log')
        return ExecResult(stdout=output.decode(errors='replace'), stderr=None, return_code=0)

    async def _resolved_services(self):
        result = await super()._run_docker_compose_command(['config', '--format', 'json'])
        resolved = json.loads(result.stdout)
        services = resolved['services']
        for name, service in services.items():
            if 'build' in service and 'image' not in service:
                service['image'] = resolved['name'] + '-' + name
        if 'main' not in services:
            raise ValueError('suite Docker environment requires main service')
        overlay = {}
        for name, service in services.items():
            if service.get('scale', 1) != 1 or service.get('deploy', {}).get('replicas', 1) != 1:
                raise ValueError('suite Docker resource accounting requires one replica per service')
            declared = service.get('deploy', {}).get('resources', {}).get('limits', {})
            cpu = service.get('cpus', declared.get('cpus'))
            memory = service.get('mem_limit', declared.get('memory'))
            if name == 'main':
                cpu = cpu if cpu is not None else self._effective_cpus
                memory = memory if memory is not None else self._effective_memory_mb * 2**20
            elif name == SIDECAR:
                cpu = cpu if cpu is not None else 0.25
                memory = memory if memory is not None else 64 * 2**20
            cpu, memory = limits(cpu, memory)
            # Preserve explicit service budgets. The overlay only pins limits,
            # with no task command, network, platform or build-context changes.
            overlay[name] = {'cpus': cpu, 'mem_limit': memory, 'memswap_limit': memory}
        path = self.trial_paths.trial_dir / 'suite-resources.json'
        atomic_json(path, {'services': overlay})
        self._suite_resource_overlay = path
        return services, overlay

    async def _run_docker_compose_command(self, command, *args, **kwargs):
        if not command or command[0] not in ('build', 'up'):
            return await super()._run_docker_compose_command(command, *args, **kwargs)
        async with self._resource_lock():
            services, resources = await self._resolved_services()
            if command[0] == 'build':
                if command != ['build']:
                    raise ValueError('unsupported suite build invocation')
                for name, service in services.items():
                    if 'build' in service:
                        budget = resources[name]
                        await self._limited_build(service, budget['cpus'], budget['mem_limit'])
                return ExecResult(stdout='', stderr=None, return_code=0)
            await self._admit(sum(v['cpus'] for v in resources.values()),
                              sum(v['mem_limit'] for v in resources.values()))
            return await super()._run_docker_compose_command(
                [command[0], '--no-build', *command[1:]], *args, **kwargs)

    async def _ensure_egress_control_sidecar_image_built(self):
        # Reuse Harbor's exact platform/content hash and normal cache tag.
        platform = await default_docker_platform()
        context = self._EGRESS_CONTROL_SIDECAR_CONTEXT_PATH
        dockerfile = self._egress_control_sidecar_dockerfile_path()
        key = docker_build_context_hash(context=context, dockerfile_path=dockerfile,
                                        build_args={}, platform=platform)
        image = _compute_image_name(self._EGRESS_CONTROL_SIDECAR_DOCKER_NAME, key)
        async with self._resource_lock():
            if not await docker_image_exists(image):
                cpu, memory = limits(self._effective_cpus, self._effective_memory_mb * 2**20)
                await self._limited_build({'image': image, 'platform': platform,
                    'build': {'context': str(context.resolve()), 'dockerfile': str(dockerfile.resolve())}}, cpu, memory)
        self._env_vars.egress_control_sidecar_image_name = image
