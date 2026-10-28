"""The explicitly authorized build-cython-ext pre-agent environment variant."""
import hashlib
from pathlib import Path
import re
import shutil

from suite.state import digest, load, tree

CASES = Path(__file__).with_name('cases')
PARENT = 'build-cython-ext'
ID = 'build-cython-ext-compat-planarity06'


def resolve(parent, descriptor):
    if parent != PARENT or not isinstance(descriptor, dict):
        raise ValueError('environment variant is not authorized for this parent')
    root = CASES / PARENT / 'variant'
    names = ('contract.json', 'Dockerfile', 'constraints.txt')
    if any((root / n).is_symlink() or not (root / n).is_file() for n in names):
        raise ValueError('missing trusted environment variant files')
    c = load(root / 'contract.json')
    if (c['variant_id'] != ID or c['original_parent_id'] != PARENT
            or c['counts_as_original_official'] is not False
            or c['dependency_phase'] != 'image_build_before_agent'
            or c['planarity_version'] != '0.6' or c['numpy_version'] != '2.3.0'
            or not re.fullmatch(r'vw-build-cython-compat@sha256:[0-9a-f]{64}', c['image'])
            or digest(root / 'Dockerfile') != c['dockerfile_sha256']
            or digest(root / 'constraints.txt') != c['constraints_sha256']):
        raise ValueError('environment variant contract drift')
    value = {'id': ID, 'contract_sha256': digest(root / 'contract.json'),
             'image': c['image'], 'counts_as_original_official': False,
             'files': {n: digest(root / n) for n in names}}
    minimal = {k: value[k] for k in ('id', 'contract_sha256')}
    if descriptor != minimal and descriptor != value:
        raise ValueError('environment variant descriptor drift')
    return value


def task_config(text, descriptor, reverse=False):
    pairs = [('name = "terminal-bench/build-cython-ext"', 'name = "terminal-bench/' + ID + '"'),
             ('docker_image = "alexgshaw/build-cython-ext:20251031"',
              'docker_image = "' + descriptor['image'] + '"')]
    for old, new in pairs:
        if reverse:
            old, new = new, old
        if text.count(old) != 1:
            raise ValueError('unexpected environment variant task configuration')
        text = text.replace(old, new)
    return text


def execution_files(official, descriptor):
    result = tree(official)
    body = task_config((Path(official) / 'task.toml').read_text(), descriptor)
    result['task.toml'] = {**result['task.toml'], 'sha256': hashlib.sha256(body.encode()).hexdigest()}
    return result


def materialize(official, output, descriptor):
    descriptor = resolve(PARENT, descriptor)
    shutil.copytree(official, output)
    path = Path(output) / 'task.toml'
    path.write_text(task_config(path.read_text(), descriptor))
    check_package(output, descriptor)
    return Path(output)


def check_package(path, descriptor):
    descriptor = resolve(PARENT, descriptor)
    catalog = load(Path(__file__).with_name('catalog.json'))
    parent = next(p for p in catalog['tasks'] if p['id'] == PARENT)
    expected = {n: {'sha256': v['sha256'], 'executable': v['mode'] == '100755'}
                for n, v in parent['files'].items()}
    actual = tree(path)
    body = task_config((Path(path) / 'task.toml').read_text(), descriptor, reverse=True)
    actual['task.toml'] = {**actual['task.toml'], 'sha256': hashlib.sha256(body.encode()).hexdigest()}
    if actual != expected:
        raise ValueError('environment variant changed official task bytes')
    return descriptor


def check_start(receipt, descriptor):
    resolve(PARENT, descriptor)
    expected = {'planarity': '0.6', 'numpy': '2.3.0',
                'constraint': '/opt/vw-compat/constraints.txt',
                'constraints_sha256': descriptor['files']['constraints.txt']}
    if receipt != {'environment_variant': descriptor, 'phase': 'before_agent', 'facts': expected}:
        raise ValueError('missing or changed pre-agent variant evidence')
