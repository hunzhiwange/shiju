"""Calibrate only the explicitly authorized pre-agent compatibility variant."""
import argparse
import asyncio
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess

from suite import diagnostics
from suite.scoring import harbor_trial, summarize
from suite.state import atomic_json, load, tree

ROOT = Path(__file__).resolve().parent


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def validate_contract(value):
    required = {'variant_id': 'build-cython-ext-compat-planarity06',
                'original_parent_id': 'build-cython-ext',
                'result_kind': 'authorized_compatibility_variant',
                'counts_as_original_official': False,
                'dependency_phase': 'image_build_before_agent',
                'planarity_version': '0.6', 'numpy_version': '2.3.0'}
    if any(type(value.get(k)) is not type(v) or value[k] != v for k, v in required.items()):
        raise ValueError('invalid compatibility variant contract')
    if not re.fullmatch(r'vw-build-cython-compat@sha256:[a-f0-9]{64}', value.get('image') or ''):
        raise ValueError('variant image must retain its pinned independent identity')
    for name, key in [('Dockerfile', 'dockerfile_sha256'), ('constraints.txt', 'constraints_sha256')]:
        if sha(ROOT / 'variant' / name) != value[key]:
            raise ValueError('variant environment recipe drift')
    return value


def validate_official(official):
    catalog = load(ROOT.parents[1] / 'catalog.json')
    parent = next(p for p in catalog['tasks'] if p['id'] == 'build-cython-ext')
    expected = {name: {'sha256': v['sha256'], 'executable': v['mode'] == '100755'}
                for name, v in parent['files'].items()}
    if tree(official) != expected:
        raise ValueError('official source drift')
    return expected


def materialize(official, wheel, output):
    contract = validate_contract(load(ROOT / 'variant/contract.json'))
    original = validate_official(official)
    if sha(wheel) != contract['wheel_sha256']:
        raise ValueError('compatibility wheel hash mismatch')
    if output.exists():
        raise ValueError('variant output already exists')
    shutil.copytree(official, output)
    config = output / 'task.toml'
    text = config.read_text()
    replacements = {
        'name = "terminal-bench/build-cython-ext"':
            'name = "terminal-bench/' + contract['variant_id'] + '"',
        'docker_image = "alexgshaw/build-cython-ext:20251031"':
            'docker_image = "' + contract['image'] + '"'}
    for before, after in replacements.items():
        if text.count(before) != 1:
            raise ValueError('unexpected official task configuration')
        text = text.replace(before, after)
    config.write_text(text)
    actual = tree(output)
    if {k: v for k, v in actual.items() if k != 'task.toml'} != {
            k: v for k, v in original.items() if k != 'task.toml'}:
        raise ValueError('variant changed official task material')
    return {'contract': contract, 'contract_sha256': sha(ROOT / 'variant/contract.json'),
            'official_files': original, 'variant_files': actual,
            'task_config_changes': ['task.name', 'environment.docker_image']}


async def environment_receipt(event, output, contract):
    if event.event.value not in ('agent-start', 'agent-end'):
        return
    project = event.trial_name + '__env'
    ids = subprocess.check_output(['docker', 'ps', '-q', '--filter',
                                  'label=com.docker.compose.project=' + project,
                                  '--filter', 'label=com.docker.compose.service=main']).decode().split()
    if len(ids) != 1:
        raise ValueError('cannot identify unique variant container')
    info = json.loads(subprocess.check_output(['docker', 'inspect', ids[0]]))[0]
    host = info['HostConfig']
    if host['NanoCpus'] != 1000000000 or host['Memory'] != 2147483648:
        raise ValueError('variant resource drift')
    code = ('import importlib.metadata as m,os,json,hashlib;from pathlib import Path;'
            'p=Path("/opt/vw-compat/constraints.txt");'
            'print(json.dumps({"planarity":m.version("planarity"),'
            '"numpy":m.version("numpy"),"constraint":os.environ.get("PIP_CONSTRAINT"),'
            '"constraints_sha256":hashlib.sha256(p.read_bytes()).hexdigest()}))')
    facts = json.loads(subprocess.check_output(['docker', 'exec', ids[0], 'python3', '-c', code]))
    if facts != {'planarity': '0.6', 'numpy': '2.3.0',
                 'constraint': '/opt/vw-compat/constraints.txt',
                 'constraints_sha256': contract['constraints_sha256']}:
        raise ValueError('agent changed compatibility dependency contract')
    atomic_json(output / (event.trial_name + '-' + event.event.value + '.json'),
                {'phase': event.event.value, 'facts': facts, 'container': ids[0],
                 'image': info['Image'], 'nano_cpus': host['NanoCpus'],
                 'memory_bytes': host['Memory'], 'mount_targets': [m['Destination'] for m in info['Mounts']]})


def calibrate(official, wheel, output):
    output.mkdir(parents=True, exist_ok=False)
    contract = validate_contract(load(ROOT / 'variant/contract.json'))
    package = output / 'packages' / contract['variant_id']
    receipt = materialize(official, wheel, package)
    receipt.update({'status': 'calibrating_variant', 'model_calls': 0, 'calibration': []})
    diagnostics.materialize(ROOT / 'case.json', output / 'diagnostics')
    tasks = [package] + sorted((output / 'diagnostics').iterdir())
    for task in tasks:
        is_parent = task == package
        for agent, reward in [('oracle', 1), ('nop', 0)]:
            name = task.name + '-' + agent
            async def hook(event):
                await environment_receipt(event, output / 'environment', contract)
            result = asyncio.run(harbor_trial(task, output / 'trials', name, agent,
                                             hook=hook if is_parent else None))
            scoring = output / 'trials' / name / 'scoring'
            summary = summarize(result)
            receipt['calibration'].append({
                'case_id': task.name, 'agent': agent,
                'result_kind': contract['result_kind'] if is_parent else 'diagnostic',
                'counts_as_original_official': False, 'result': result, 'summary': summary,
                'first_score': load(scoring / 'first.json') if (scoring / 'first.json').exists() else {},
                'evidence_root': str(scoring.resolve()), 'evidence_files': tree(scoring)})
            atomic_json(output / 'variant-calibration.json', receipt)
            print(name, summary, flush=True)
            if summary['score_status'] != 'scored' or summary['reward'] != reward:
                raise ValueError('variant calibration failed; original scores are unchanged')
    validate_official(official)
    receipt['status'] = 'calibrated_variant'
    receipt['environment_evidence'] = tree(output / 'environment')
    atomic_json(output / 'variant-calibration.json', receipt)
    return receipt


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--official', type=Path, required=True)
    parser.add_argument('--wheel', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    calibrate(args.official.resolve(), args.wheel.resolve(), args.output.resolve())
