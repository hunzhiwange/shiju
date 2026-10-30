"""The explicitly authorized pre-agent compatibility variants."""
import hashlib
import json
from pathlib import Path
import re
import shutil
import shlex

from suite.state import digest, load, tree
from suite import crack_variant

CASES = Path(__file__).with_name('cases')
PARENT = 'build-cython-ext'
ID = 'build-cython-ext-compat-planarity06'

CMAKE_PARENT = 'mcmc-sampling-stan'
CMAKE_ID = 'mcmc-sampling-stan-compat-cmake3283'
CMAKE_BASE = 'alexgshaw/mcmc-sampling-stan@sha256:073fc36a3347b4f6b43bd6e98339542c41ed531c771af8bee40ba10f00cfabaf'
STAN_ID = 'mcmc-sampling-stan-compat-remotes-never'
STAN_BASE = 'vw-mcmc-stan-cmake-compat@sha256:a6c991f2a0f6ba19abdccbd006bef08b40b9c380549d59aa29113c8a9df95559'
STAN_BASE_CONTRACT = '0bcab5451c3a5c90a693febef793dcd67b2b2ab315b33f52f6f498f4717e803c'
STAN_RECIPE = ('FROM ' + STAN_BASE + '\nENV R_REMOTES_UPGRADE=never\n'
               'LABEL vw.original-parent="mcmc-sampling-stan" vw.counts-as-original-official="false"\n')
# Exact seven-package closure authorized for task 56; no arbitrary packages.
CMAKE_PACKAGES = {
    'cmake': {'version': '3.28.3-1build7', 'architecture': 'amd64', 'filename': 'cmake_3.28.3-1build7_amd64.deb', 'sha256': '4b0a7f8c0daf27b26b46997d994ae5d1ee7a3d11dfcda9f7627bb1462c162295'},
    'cmake-data': {'version': '3.28.3-1build7', 'architecture': 'all', 'filename': 'cmake-data_3.28.3-1build7_all.deb', 'sha256': '20a3b644211ce82f35c24f1f5052199adeb0ac159978a8740fd6c0959611557f'},
    'libarchive13t64': {'version': '3.7.2-2', 'architecture': 'amd64', 'filename': 'libarchive13t64_3.7.2-2_amd64.deb', 'sha256': '90f7f4692ac6b2ce1755c0f2e67f9655e288d764ef9d70dba7c3f7ae8b8a5b4b'},
    'libjsoncpp25': {'version': '1.9.5-6build1', 'architecture': 'amd64', 'filename': 'libjsoncpp25_1.9.5-6build1_amd64.deb', 'sha256': '8efea5b75952f3ad1c40e6b5b31de687c297ea29f87f8b63efebb55e13467211'},
    'librhash0': {'version': '1.4.3-3build1', 'architecture': 'amd64', 'filename': 'librhash0_1.4.3-3build1_amd64.deb', 'sha256': 'e9ee69963ff1a56378b9c5ffdd21ea0feaec9647c522349b7612b25103490528'},
    'libuv1t64': {'version': '1.48.0-1.1build1', 'architecture': 'amd64', 'filename': 'libuv1t64_1.48.0-1.1build1_amd64.deb', 'sha256': '72a1856b30af1e533494ab90742a2446f4491b15ddebcade6661a6e8d61b4359'},
    'libxml2': {'version': '2.9.14+dfsg-1.3ubuntu3', 'architecture': 'amd64', 'filename': 'libxml2_2.9.14+dfsg-1.3ubuntu3_amd64.deb', 'sha256': '8c4efd7abe155df3cf0f9b64d659f2d866215785f8e8c44234fbb341d58cc967'},
}


def parent_for(descriptor):
    if not isinstance(descriptor, dict):
        raise ValueError('invalid environment variant descriptor')
    if descriptor.get('id') == ID:
        return PARENT
    if descriptor.get('id') in (CMAKE_ID, STAN_ID):
        return CMAKE_PARENT
    if descriptor.get('id') == crack_variant.ID:
        return crack_variant.PARENT
    raise ValueError('environment variant is not authorized')


def resolve_descriptor(descriptor):
    return resolve(parent_for(descriptor), descriptor)


def cmake_metadata(root, contract):
    lock = load(root / 'deb-lock.json')
    packages = {}
    for row in lock['packages']:
        name = row['package']
        if name in packages:
            raise ValueError('duplicate CMake package')
        packages[name] = {k: row[k] for k in ('version', 'architecture', 'filename', 'sha256')}
    sums = ''.join(row['sha256'] + '  ' + row['filename'] + '\n' for row in CMAKE_PACKAGES.values())
    inventory = load(root / 'base-packages.json')
    if (packages != CMAKE_PACKAGES or lock['base'] != CMAKE_BASE
            or lock['existing_packages_upgraded'] != []
            or (root / 'SHA256SUMS').read_text() != sums
            or contract['base_image'] != CMAKE_BASE or contract['cmake_version'] != '3.28.3'
            or not isinstance(inventory, dict) or not inventory
            or set(inventory) & set(CMAKE_PACKAGES)
            or any(not isinstance(v, dict) or set(v) != {'version', 'architecture'}
                   or not all(isinstance(x, str) and x for x in v.values()) for v in inventory.values())
            or [line.strip() for line in (root / 'Dockerfile').read_text().splitlines()
                if line.strip().upper().startswith('FROM ')] != ['FROM ' + CMAKE_BASE]
            or digest(root / 'deb-lock.json') != contract['packages_lock_sha256']
            or digest(root / 'SHA256SUMS') != contract['sha256sums_sha256']
            or digest(root / 'base-packages.json') != contract['base_packages_sha256']):
        raise ValueError('CMake variant exceeds authorized package/base closure')
    return inventory


def resolve(parent, descriptor):
    if parent == CMAKE_PARENT and isinstance(descriptor, dict) and descriptor.get('id') == STAN_ID:
        return resolve_stan(descriptor)
    if parent == crack_variant.PARENT and parent_for(descriptor) == parent:
        return crack_variant.resolve(CASES / parent / 'variant', descriptor)
    if parent not in (PARENT, CMAKE_PARENT) or parent_for(descriptor) != parent:
        raise ValueError('environment variant is not authorized for this parent')
    root = CASES / parent / 'variant'
    names = (('contract.json', 'Dockerfile', 'constraints.txt') if parent == PARENT else
             ('contract.json', 'Dockerfile', 'deb-lock.json', 'SHA256SUMS', 'base-packages.json'))
    if any((root / n).is_symlink() or not (root / n).is_file() for n in names):
        raise ValueError('missing trusted environment variant files')
    c = load(root / 'contract.json')
    selected_id = ID if parent == PARENT else CMAKE_ID
    prefix = 'vw-build-cython-compat' if parent == PARENT else 'vw-mcmc-stan-cmake-compat'
    if (c['variant_id'] != selected_id or c['original_parent_id'] != parent
            or c['counts_as_original_official'] is not False
            or c['dependency_phase'] != 'image_build_before_agent'
            or not re.fullmatch(prefix + r'@sha256:[0-9a-f]{64}', c['image'])
            or digest(root / 'Dockerfile') != c['dockerfile_sha256']):
        raise ValueError('environment variant contract drift')
    if parent == PARENT:
        if (c['planarity_version'] != '0.6' or c['numpy_version'] != '2.3.0'
                or digest(root / 'constraints.txt') != c['constraints_sha256']):
            raise ValueError('environment variant contract drift')
    else:
        cmake_metadata(root, c)
    value = {'id': selected_id, 'contract_sha256': digest(root / 'contract.json'),
             'image': c['image'], 'counts_as_original_official': False,
             'files': {n: digest(root / n) for n in names}}
    minimal = {k: value[k] for k in ('id', 'contract_sha256')}
    if descriptor != minimal and descriptor != value:
        raise ValueError('environment variant descriptor drift')
    return value


def stan_base_descriptor():
    return resolve(CMAKE_PARENT, {'id': CMAKE_ID, 'contract_sha256': STAN_BASE_CONTRACT})


def resolve_stan(descriptor):
    root = CASES / CMAKE_PARENT / 'stan-variant'
    names = ('contract.json', 'Dockerfile')
    if any((root / n).is_symlink() or not (root / n).is_file() for n in names):
        raise ValueError('missing trusted Stan upgrade-policy variant files')
    contract = load(root / 'contract.json')
    base = stan_base_descriptor()
    if (contract.get('variant_id') != STAN_ID
            or contract.get('original_parent_id') != CMAKE_PARENT
            or contract.get('counts_as_original_official') is not False
            or contract.get('dependency_phase') != 'image_build_before_agent'
            or contract.get('base_image') != STAN_BASE or base['image'] != STAN_BASE
            or contract.get('base_variant') != base
            or contract.get('environment') != {'R_REMOTES_UPGRADE': 'never'}
            or not re.fullmatch(r'vw-mcmc-stan-remotes-never@sha256:[0-9a-f]{64}', contract.get('image', ''))
            or (root / 'Dockerfile').read_text() != STAN_RECIPE
            or contract.get('dockerfile_sha256') != digest(root / 'Dockerfile')):
        raise ValueError('Stan variant exceeds authorized upgrade-policy boundary')
    value = {'id': STAN_ID, 'contract_sha256': digest(root / 'contract.json'),
             'image': contract['image'], 'counts_as_original_official': False,
             'files': {n: digest(root / n) for n in names}}
    if descriptor not in (value, {k: value[k] for k in ('id', 'contract_sha256')}):
        raise ValueError('Stan variant descriptor drift')
    return value


def task_config(text, descriptor, reverse=False):
    parent = parent_for(descriptor)
    pairs = [('name = "terminal-bench/' + parent + '"', 'name = "terminal-bench/' + descriptor['id'] + '"'),
             ('docker_image = "alexgshaw/' + parent + ':20251031"',
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
    descriptor = resolve_descriptor(descriptor)
    shutil.copytree(official, output)
    path = Path(output) / 'task.toml'
    path.write_text(task_config(path.read_text(), descriptor))
    check_package(output, descriptor)
    return Path(output)


def check_package(path, descriptor):
    descriptor = resolve_descriptor(descriptor)
    catalog = load(Path(__file__).with_name('catalog.json'))
    parent = next(p for p in catalog['tasks'] if p['id'] == parent_for(descriptor))
    expected = {n: {'sha256': v['sha256'], 'executable': v['mode'] == '100755'}
                for n, v in parent['files'].items()}
    actual = tree(path)
    body = task_config((Path(path) / 'task.toml').read_text(), descriptor, reverse=True)
    actual['task.toml'] = {**actual['task.toml'], 'sha256': hashlib.sha256(body.encode()).hexdigest()}
    if actual != expected:
        raise ValueError('environment variant changed official task bytes')
    return descriptor


def expected_facts(descriptor):
    descriptor = resolve_descriptor(descriptor)
    if descriptor['id'] == STAN_ID:
        return {**expected_facts(stan_base_descriptor()), 'remotes_upgrade': 'never'}
    if parent_for(descriptor) == crack_variant.PARENT:
        root = CASES / crack_variant.PARENT / 'variant'
        return crack_variant.expected_facts(root, load(root / 'contract.json'))
    if parent_for(descriptor) == PARENT:
        return {'planarity': '0.6', 'numpy': '2.3.0',
                'constraint': '/opt/vw-compat/constraints.txt',
                'constraints_sha256': descriptor['files']['constraints.txt']}
    root = CASES / CMAKE_PARENT / 'variant'
    inventory = cmake_metadata(root, load(root / 'contract.json'))
    return {'cmake': 'cmake version 3.28.3',
            'packages': {name: {k: row[k] for k in ('version', 'architecture')}
                         for name, row in CMAKE_PACKAGES.items()},
            'base_inventory_sha256': hashlib.sha256(json.dumps(inventory, sort_keys=True,
                                        separators=(',', ':')).encode()).hexdigest()}


def probe_code(descriptor):
    if parent_for(descriptor) == PARENT:
        return ('import importlib.metadata as m,os,json,hashlib;from pathlib import Path;'
                'p=Path("/opt/vw-compat/constraints.txt");'
                'print(json.dumps({"planarity":m.version("planarity"),'
                '"numpy":m.version("numpy"),"constraint":os.environ.get("PIP_CONSTRAINT"),'
                '"constraints_sha256":hashlib.sha256(p.read_bytes()).hexdigest()}))')
    # Read-only facts before Agent. No apt, dpkg installation, network or oracle.
    environment_fact = ("'remotes_upgrade':os.environ.get('R_REMOTES_UPGRADE'),"
                        if descriptor['id'] == STAN_ID else '')
    return ("import subprocess,json,hashlib,os\n"
            "raw=subprocess.check_output(['dpkg-query','-W','-f=${Package}\\t${Version}\\t${Architecture}\\n']).decode()\n"
            "installed={}\n"
            "for line in raw.splitlines():\n"
            " name,version,arch=line.split('\\t');assert name not in installed;installed[name]={'version':version,'architecture':arch}\n"
            "names=" + repr(list(CMAKE_PACKAGES)) + "\n"
            "packages={name:installed.pop(name) for name in names}\n"
            "print(json.dumps({'cmake':subprocess.check_output(['cmake','--version']).decode().splitlines()[0],"
            "'packages':packages," + environment_fact + "'base_inventory_sha256':hashlib.sha256(json.dumps(installed,sort_keys=True,separators=(',',':')).encode()).hexdigest()}))")


def check_start(receipt, descriptor):
    expected = expected_facts(descriptor)
    if receipt != {'environment_variant': descriptor, 'phase': 'before_agent', 'facts': expected}:
        raise ValueError('missing or changed pre-agent variant evidence')


def probe_command(descriptor):
    descriptor = resolve_descriptor(descriptor)
    if parent_for(descriptor) == crack_variant.PARENT:
        root = CASES / crack_variant.PARENT / 'variant'
        return crack_variant.probe_command(root, load(root / 'contract.json'))
    return 'python3 -c ' + shlex.quote(probe_code(descriptor))
