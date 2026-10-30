"""Run the unscored A+B+C lifecycle probe in a bounded, offline container."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import time
import uuid

from materialize import IMAGE, materialize
from suite.docker import LOCK, MAX_CPU, MAX_MEMORY, active_resources
from suite.state import atomic_json, digest, load, locked


def run(runtime_path, output):
    runtime = load(runtime_path)
    output.mkdir(parents=True, exist_ok=False)
    task = output / 'helper-package'
    identity = materialize(Path(runtime['source_root']) / 'tasks' / 'filter-js-from-html', task)
    policy = next(t['recovery']['bootstrap'] for t in runtime['tasks']
                  if t['taskId'] == 'filter-js-from-html')
    inputs = output / 'inputs'
    inputs.mkdir()
    for item in policy['files']:
        if digest(item['source']) != item['sha256']:
            raise ValueError('verifier dependency identity drift')
        shutil.copyfile(item['source'], inputs / item['target'])
    name = 'vw-task36-compat-probe-' + uuid.uuid4().hex
    command = ['docker', 'run', '--pull', 'never', '--name', name, '--platform', 'linux/amd64',
               '--network', 'none', '--cpus', '1', '--memory', '1g', '--memory-swap', '1g',
               '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges',
               '--mount', f'type=bind,src={inputs},dst=/inputs,readonly',
               '--mount', f'type=bind,src={Path(__file__).resolve().parent},dst=/control,readonly',
               '--mount', f'type=bind,src={task / "tests"},dst=/helper,readonly']
    for key, value in policy['env'].items():
        command += ['-e', key + '=' + value]
    command += ['--entrypoint', 'sh', IMAGE, '-c',
                'mkdir -p /opt/vw-verifier-deps && cp /inputs/* /opt/vw-verifier-deps/ && '
                'sh /opt/vw-verifier-deps/setup.sh && /root/.local/bin/uvx -p 3.13 '
                '-w pytest==8.4.1 -w selenium==4.38.0 -w bs4==0.0.2 '
                '-w pytest-json-ctrf==0.3.5 python /control/probe.py /helper/test_outputs.py']
    receipt = {'command': command, 'identity': identity,
               'kind': 'synthetic compatibility lifecycle probe; not scoring',
               'probe_sha256': digest(Path(__file__).with_name('probe.py'))}
    started = time.monotonic()
    # Release the lock while waiting for capacity. Once admitted, keep it for
    # this short probe through cleanup so its reservation remains exclusive.
    while True:
        with locked(LOCK, blocking=True):
            ids = subprocess.check_output(['docker', 'ps', '-q']).decode().split()
            containers = json.loads(subprocess.check_output(['docker', 'inspect', *ids])) if ids else []
            cpu, memory = active_resources(containers)
            if cpu + 1 <= MAX_CPU and memory + 2**30 <= MAX_MEMORY:
                try:
                    result = subprocess.run(command, capture_output=True, text=True, timeout=180)
                    receipt.update(exit_code=result.returncode, stdout=result.stdout, stderr=result.stderr)
                except subprocess.TimeoutExpired:
                    receipt.update(exit_code=None, timed_out=True)
                finally:
                    result = subprocess.run(['docker', 'inspect', name], capture_output=True, text=True, timeout=15)
                    receipt['inspect'] = json.loads(result.stdout) if result.returncode == 0 else result.stderr
                    result = subprocess.run(['docker', 'rm', '-f', name], capture_output=True, timeout=15)
                    receipt['cleanup_exit_code'] = result.returncode
                    receipt['elapsed_sec'] = time.monotonic() - started
                    atomic_json(output / 'probe-receipt.json', receipt)
                break
        if time.monotonic() - started > 1800:
            raise ValueError('suite capacity wait expired before lifecycle probe')
        time.sleep(5)
    if (receipt['exit_code'] != 0 or receipt['cleanup_exit_code'] != 0
            or 'COMPAT_LIFECYCLE_PROBE_PASSED' not in receipt.get('stdout', '')):
        raise ValueError('lifecycle probe failed; see probe-receipt.json')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    run(args.runtime.resolve(), args.output.resolve())
