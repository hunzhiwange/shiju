"""Calibrate an explicitly separate network-cache identity, without model calls.

Requires eight standard child receipts and the unchanged frozen upstream tree.
Never writes original preparation.json or counts as original-network readiness.
"""
import argparse
import asyncio
import ast
from functools import partial
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import uuid

from suite import diagnostics
from suite import docker as suite_docker
from suite.scoring import harbor_trial, summarize
from suite.state import atomic_json, digest, load, locked, tree

CASE = Path(__file__).resolve().parents[1]
CATALOG = CASE.parents[1] / 'catalog.json'
IMAGE = 'sha256:929a6d631b592c82e0d0f5d4a65cf03c946dc554cfc9bb09916e82076e7d3215'
VARIANT = 'caffe-cifar-10-network-cache'
MIN_FREE = 20490110579


def checked_source(source):
    parent = next(t for t in load(CATALOG)['tasks'] if t['id'] == 'caffe-cifar-10')
    expected = {n: {'sha256': v['sha256'], 'executable': v['mode'] == '100755'}
                for n, v in parent['files'].items()}
    if tree(source) != expected:
        raise ValueError('frozen upstream source drift')
    return expected


def checked_children(receipt):
    rows = load(receipt)
    wanted = [(f'caffe-cifar-10-m{i}', agent, reward) for i in range(1, 5)
              for agent, reward in [('oracle', 1), ('nop', 0)]]
    if [(r['task'], r['agent'], r['summary']['reward']) for r in rows] != wanted:
        raise ValueError('eight successful child calibrations required')
    with tempfile.TemporaryDirectory() as directory:
        generated = Path(directory) / 'tasks'
        diagnostics.materialize(CASE / 'case.json', generated)
        for row in rows:
            config = row['result']['config']
            trial = Path(config['trials_dir']) / config['trial_name']
            if (summarize(row['result']) != row['summary']
                    or row['summary']['score_status'] != 'scored'
                    or tree(generated / row['task']) != row['hashes']
                    or tree(Path(config['task']['path'])) != row['hashes']
                    or load(trial / 'scoring/first.json')['rewards']['reward'] != row['summary']['reward']):
                raise ValueError('child package/scorer evidence drift')
    return {'sha256': digest(receipt), 'trials': 8,
            'case_files': diagnostics.validate(CASE / 'case.json')['files']}


def containers(name):
    return subprocess.check_output(['docker', 'ps', '-aq', '--filter',
                                   'label=com.docker.compose.project=' + name + '__env'], timeout=20).decode().split()


def archive_and_remove(name, output):
    for identity in containers(name):
        inspected = json.loads(subprocess.check_output(['docker', 'inspect', identity], timeout=20))[0]
        atomic_json(output / (name + '-' + identity[:12] + '-inspect.json'), inspected)
        artifacts = output / (name + '-artifacts')
        artifacts.mkdir(exist_ok=True)
        for remote in ('training_output.txt', 'Makefile.config',
                       'examples/cifar10/cifar10_quick_solver.prototxt',
                       'examples/cifar10/cifar10_quick_train_test.prototxt',
                       'examples/cifar10/cifar10_quick_iter_500.caffemodel'):
            subprocess.run(['docker', 'cp', identity + ':/app/caffe/' + remote,
                            str(artifacts / Path(remote).name)], capture_output=True, timeout=30)
        atomic_json(artifacts / 'hashes.json', tree(artifacts))
        subprocess.run(['docker', 'rm', '-f', identity], capture_output=True, check=True, timeout=30)


async def wait_capacity():
    reported = False
    while True:
        # Inspect under the same lock used by SuiteDockerEnvironment admission.
        with locked(suite_docker.LOCK, blocking=True):
            ids = subprocess.check_output(['docker', 'ps', '-q'], timeout=20).decode().split()
            running = json.loads(subprocess.check_output(['docker', 'inspect', *ids], timeout=20)) if ids else []
            cpu, memory = suite_docker.active_resources(running)
        if cpu == 0 and memory == 0:
            return
        if not reported:
            print('Waiting for exclusive 4 CPU / 8GiB window', flush=True)
            reported = True
        await asyncio.sleep(10)


async def calibrate(source, archive, children, output):
    source_files = checked_source(source)
    child_evidence = checked_children(children)
    if shutil.disk_usage(output.parent).free < MIN_FREE:
        raise ValueError('parent requires at least 19.08GiB free host space')
    output.mkdir(parents=True, exist_ok=False)
    proxy_script = Path(__file__).with_name('cache_proxy.py')
    proxy = subprocess.Popen([sys.executable, str(proxy_script), '--archive', str(archive),
                              '--receipt', str(output / 'proxy-instance.json')],
                             stdout=subprocess.PIPE, stderr=(output / 'proxy.stderr').open('wb'))
    name = None
    previous_platform = os.environ.get("DOCKER_DEFAULT_PLATFORM")
    os.environ["DOCKER_DEFAULT_PLATFORM"] = "linux/amd64"
    old_locked = suite_docker.locked
    # Scheduling only: wait on the identical lock instead of failing contention.
    # All original resource admission checks still run; no shared source edit.
    suite_docker.locked = partial(locked, blocking=True)
    try:
        if proxy.stdout.readline().strip() != b'ready':
            raise ValueError('cache proxy failed archive verification/startup')
        instance = load(output / 'proxy-instance.json')
        if instance['pid'] != proxy.pid:
            raise ValueError('proxy process identity mismatch')
        task = output / VARIANT
        shutil.copytree(source, task)
        config = task / 'task.toml'
        text = config.read_text()
        if text.count('[environment.env]') != 1:
            raise ValueError('unexpected upstream environment configuration')
        proxy_url = 'http://host.docker.internal:' + str(instance['port'])
        config.write_text(text.replace('[environment.env]', '[environment.env]\n'
                          + 'http_proxy = ' + json.dumps(proxy_url) + '\n'
                          + 'https_proxy = ' + json.dumps(proxy_url)))
        count = sum(isinstance(n, ast.FunctionDef) and n.name.startswith('test_')
                    for n in ast.parse((source / 'tests/test_outputs.py').read_text()).body)
        run_id = uuid.uuid4().hex[:10]
        receipt = {'run_id': run_id, 'status': 'incomplete', 'variant': VARIANT,
                   'counts_as_original_official': False, 'model_calls': 0, 'platform': 'linux/amd64',
                   'source_files': source_files, 'execution_files': tree(task),
                   'children': child_evidence, 'proxy': instance,
                   'proxy_script_sha256': digest(proxy_script), 'driver_sha256': digest(__file__),
                   'agent_timeout_sec': 3600, 'verifier_timeout_sec': 1200,
                   'official_test_count': count, 'calibration': []}
        atomic_json(output / 'compatibility-preparation.json', receipt)
        for agent, expected in [('oracle', 1), ('nop', 0)]:
            for attempt in range(1, 61):
                await wait_capacity()
                if shutil.disk_usage(output).free < MIN_FREE:
                    raise ValueError('parent disk admission no longer satisfied')
                name = VARIANT + '-' + run_id + '-' + agent + '-admission-' + str(attempt)
                async def hook(event):
                    if tree(source) != source_files or tree(task) != receipt['execution_files']:
                        raise ValueError('source/execution package drift')
                    if event.event.value == 'agent-start':
                        ids = containers(name)
                        if len(ids) != 1:
                            raise ValueError('unexpected parent container count')
                        inspected = json.loads(subprocess.check_output(['docker', 'inspect', ids[0]], timeout=20))[0]
                        host = inspected['HostConfig']
                        if (inspected['Image'] != IMAGE or host['NanoCpus'] != 4000000000
                                or host['Memory'] != 8589934592
                                or any(not m['Destination'].startswith('/logs/') for m in inspected['Mounts'])):
                            raise ValueError('parent image/resource/isolation drift')
                        disk = subprocess.check_output(['docker', 'exec', ids[0], 'df', '-Pk', '/'], timeout=20).decode()
                        if int(disk.splitlines()[-1].split()[3]) * 1024 < MIN_FREE:
                            raise ValueError('Docker VM disk admission failed')
                        atomic_json(output / (name + '-before-agent.json'), inspected)
                        atomic_json(output / (name + '-disk.json'), {'vm_df': disk, 'host_free_bytes': shutil.disk_usage(output).free})
                result = await harbor_trial(task, output / 'calibration', name, agent, hook=hook)
                archive_and_remove(name, output)
                error = result.get('exception_info') or {}
                if error.get('exception_type') in ('BlockingIOError', 'ValueError') and (
                        'Resource temporarily unavailable' in str(error)
                        or 'suite Docker capacity exceeds' in str(error)):
                    # No candidate/scorer ran. Preserve this trial, then await admission.
                    print('Admission changed before startup; waiting again', flush=True)
                    await asyncio.sleep(5)
                    continue
                break
            summary = summarize(result)
            trial = output / 'calibration' / name
            logs = (trial / 'verifier/test-stdout.txt').read_text() if (trial / 'verifier/test-stdout.txt').exists() else ''
            valid = (summary['score_status'] == 'scored' and summary['reward'] == expected
                     and f'{count} ' + ('passed' if expected else 'failed') in logs)
            row = {'agent': agent, 'summary': summary, 'result': result, 'valid': valid,
                   'trial_path': str(trial), 'scoring_files': tree(trial / 'scoring'),
                   'verifier_files': tree(trial / 'verifier')}
            receipt['calibration'].append(row)
            atomic_json(output / 'compatibility-preparation.json', receipt)
            print(agent, summary, 'valid', valid, flush=True)
            if not valid:
                raise ValueError('parent compatibility calibration failed')
        receipt['status'] = 'compatibility_calibrated'
        atomic_json(output / 'compatibility-preparation.json', receipt)
    finally:
        suite_docker.locked = old_locked
        if previous_platform is None:
            os.environ.pop("DOCKER_DEFAULT_PLATFORM", None)
        else:
            os.environ["DOCKER_DEFAULT_PLATFORM"] = previous_platform
        try:
            if name:
                archive_and_remove(name, output)
        finally:
            proxy.terminate()
            proxy.communicate(timeout=10)
            atomic_json(output / 'proxy-cleanup.json', {'pid': proxy.pid, 'returncode': proxy.returncode})


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ('source', 'archive', 'children', 'output'):
        parser.add_argument('--' + flag, required=True, type=Path)
    args = parser.parse_args()
    asyncio.run(calibrate(args.source.resolve(), args.archive.resolve(), args.children.resolve(), args.output.resolve()))
