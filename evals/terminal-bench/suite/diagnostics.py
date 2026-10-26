"""Explicit diagnostic format and Harbor packages (never imports parent solutions).

case.json has schema_version=1, parent_id, a digest-pinned Python 3 image,
policy=POLICY, and four children m1..m4. Each child declares instruction and
checker paths, fixtures/oracle/gold file maps (destination: source), and >=3
negatives with distinct id/files. All sources are regular files beneath the
case directory; fixtures must be synthetic. check.py receives candidate and
oracle directories as argv[1:]; exit 0 means pass, 1 means rejection, other
codes mean checker failure. Gold is an independent artifact, never generated
from agent output. Checkers may execute programs or probe services themselves.

Only explicit fixtures enter environment/; tests and solution are injected by
Harbor in their respective phases. Base images must themselves be reviewed
for hidden material by per-case preparation. Calibration uses fresh, offline,
resource-limited Docker containers, no parent task or repository mount.
validation.json binds every declared byte and this implementation; it records
local fixture calibration, not official scoring or model readiness.
"""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import shlex
import shutil
import subprocess
import tempfile
import uuid

POLICY = {'model': 'token-plan/qwen3.8-flash', 'max_output_tokens': 32000,
          'input_window_tokens': 200000, 'agent_timeout_sec': 900,
          'verifier_timeout_sec': 120, 'max_turns': 24, 'max_tools': 32,
          'attempts': 1, 'fresh_container': True}


# Keep an unhandled checker exception distinct from an intentional exit(1).
CHECKER_BOOTSTRAP = (
    "import runpy,sys\n"
    "sys.argv=['/tests/check.py','/app','/tests/oracle']\n"
    "try:\n runpy.run_path('/tests/check.py',run_name='__main__')\n"
    "except SystemExit:\n raise\n"
    "except BaseException:\n sys.exit(2)\n"
)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def keys(value, expected):
    if not isinstance(value, dict) or set(value) != set(expected.split()):
        raise ValueError('unexpected or missing diagnostic fields')


def relative(value):
    if (not isinstance(value, str) or not value or '\\' in value
            or any(part in ('', '.', '..') for part in value.split('/'))
            or PurePosixPath(value).is_absolute()
            or not re.fullmatch(r'[A-Za-z0-9_.\-/]+', value)):
        raise ValueError('unsafe relative path')
    return value


def source(root, value):
    value = relative(value)
    path = root / value
    if any((root / Path(*Path(value).parts[:n])).is_symlink()
           for n in range(1, len(Path(value).parts) + 1)):
        raise ValueError('symlink source forbidden')
    if not path.is_file() or not path.resolve().is_relative_to(root.resolve()):
        raise ValueError('missing or external source')
    return path


def file_map(root, mapping, *, empty=False):
    if not isinstance(mapping, dict) or (not mapping and not empty):
        raise ValueError('explicit fixture files required')
    destinations = set()
    for target, name in mapping.items():
        relative(target)
        if any(target.startswith(p + '/') or p.startswith(target + '/') for p in destinations):
            raise ValueError('overlapping destination paths')
        destinations.add(target)
        source(root, name)
    return list(mapping.values())


def load_case(path):
    path = Path(path)
    root = path.parent
    case = json.loads(path.read_text())
    keys(case, 'schema_version parent_id image policy children')
    if type(case['schema_version']) is not int or case['schema_version'] != 1:
        raise ValueError('unsupported schema version')
    if not isinstance(case['parent_id'], str) or not re.fullmatch('[a-z0-9][a-z0-9-]*', case['parent_id']):
        raise ValueError('invalid parent ID')
    if not isinstance(case['image'], str) or not re.fullmatch(r'[a-zA-Z0-9_./:-]+@sha256:[a-f0-9]{64}', case['image']):
        raise ValueError('digest-pinned image required')
    if json.dumps(case['policy'], sort_keys=True) != json.dumps(POLICY, sort_keys=True):
        raise ValueError('diagnostic policy drift')
    children = case['children']
    if not isinstance(children, list) or len(children) != 4:
        raise ValueError('exactly four children required')
    ids = []
    declared = []
    for child in children:
        keys(child, 'id instruction checker fixtures oracle gold negatives')
        ids.append(child['id'])
        public = [child['instruction']] + file_map(root, child['fixtures'])
        hidden = [child['checker']] + file_map(root, child['oracle']) + file_map(root, child['gold'])
        protected = hidden.copy()
        negatives = child['negatives']
        if not isinstance(negatives, list) or len(negatives) < 3:
            raise ValueError('three meaningful negative fixtures required')
        negative_ids = []
        signatures = []
        gold = {k: digest(source(root, v)) for k, v in child['gold'].items()}
        for negative in negatives:
            keys(negative, 'id files')
            if not isinstance(negative['id'], str) or not re.fullmatch('[a-z0-9-]+', negative['id']):
                raise ValueError('invalid negative ID')
            if negative['id'] in ('gold', 'nop'):
                raise ValueError('reserved fixture identity')
            negative_ids.append(negative['id'])
            hidden += file_map(root, negative['files'])
            signature = {k: digest(source(root, v)) for k, v in negative['files'].items()}
            if signature == gold or signature in signatures:
                raise ValueError('duplicate or ineffective negative artifact')
            signatures.append(signature)
        if len(set(negative_ids)) != len(negative_ids):
            raise ValueError('duplicate negative ID')
        for name in public + hidden:
            source(root, name)
        # Reject direct or byte-identical leaks, including instruction leakage.
        hidden_hashes = {digest(source(root, name)) for name in protected}
        if any(digest(source(root, name)) in hidden_hashes for name in public):
            raise ValueError('hidden material in agent-visible files')
        declared += public + hidden
    if sorted(ids) != ['m1', 'm2', 'm3', 'm4']:
        raise ValueError('child IDs must be unique m1..m4')
    hashes = {name: digest(source(root, name)) for name in sorted(set(declared))}
    hashes[path.name] = digest(path)
    # Public bytes in any child must not reveal another child's gold/oracle.
    protected_hashes = set()
    for child in children:
        protected_hashes.update(digest(source(root, name))
                                for name in [child['checker'], *child['oracle'].values(),
                                             *child['gold'].values()])
    for child in children:
        if any(digest(source(root, name)) in protected_hashes
               for name in [child['instruction'], *child['fixtures'].values()]):
            raise ValueError('cross-child hidden material leakage')
    return case, hashes


def copy_files(root, mapping, destination):
    for target, name in mapping.items():
        output = destination / target
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(source(root, name), output)


def run_checker(image, root, child, candidate):
    with tempfile.TemporaryDirectory(prefix='vw-diagnostic-') as directory:
        work = Path(directory)
        app = work / 'app'
        tests = work / 'tests'
        app.mkdir()
        tests.mkdir()
        copy_files(root, child['fixtures'], app)
        copy_files(root, candidate, app)
        copy_files(root, child['oracle'], tests / 'oracle')
        shutil.copyfile(source(root, child['checker']), tests / 'check.py')
        name = 'vw-diagnostic-' + uuid.uuid4().hex
        command = ['docker', 'run', '--pull', 'never', '--rm', '--name', name,
                   '--network', 'none', '--cpus', '1', '--memory', '512m',
                   '--pids-limit', '128', '--cap-drop', 'ALL',
                   '--security-opt', 'no-new-privileges', '--read-only',
                   '--tmpfs', '/tmp:rw,size=64m', '--workdir', '/app',
                   '--mount', f'type=bind,src={app},dst=/app',
                   '--mount', f'type=bind,src={tests},dst=/tests,readonly',
                   '--entrypoint', 'python3', image,
                   '-c', CHECKER_BOOTSTRAP]
        try:
            result = subprocess.run(command, capture_output=True, timeout=120)
        finally:
            # Also stop a container if client timeout/interruption occurs.
            subprocess.run(['docker', 'rm', '-f', name], capture_output=True, timeout=30)
        if result.returncode not in (0, 1):
            raise ValueError(f'checker/infrastructure failure: exit {result.returncode}')
        return result.returncode


def validate(path, run_fixtures=False):
    path = Path(path)
    case, hashes = load_case(path)
    receipt = {'schema_version': 1, 'parent_id': case['parent_id'],
               'implementation_sha256': digest(__file__), 'files': hashes,
               'image': case['image'], 'policy': POLICY.copy(), 'checks': []}
    if run_fixtures:
        for child in case['children']:
            candidates = [('gold', child['gold'], 0), ('nop', {}, 1)]
            candidates += [(n['id'], n['files'], 1) for n in child['negatives']]
            for identity, files, expected in candidates:
                code = run_checker(case['image'], path.parent, child, files)
                if code != expected:
                    raise ValueError(f"{child['id']} {identity}: checker calibration failed")
                receipt['checks'].append({'child_id': child['id'], 'fixture': identity,
                                          'exit_code': code})
    return receipt


def validate_all(catalog, cases_root, verify_receipts=False):
    tasks = json.loads(Path(catalog).read_text())['tasks']
    ids = [task['id'] for task in tasks]
    if not ids or len(set(ids)) != len(ids):
        raise ValueError('invalid catalog IDs')
    paths = list(Path(cases_root).glob('*/case.json'))
    cases = {load_case(path)[0]['parent_id']: path for path in paths}
    if len(cases) != len(paths) or set(cases) != set(ids):
        raise ValueError('catalog/case coverage mismatch')
    for parent, path in cases.items():
        if path.parent.name != parent:
            raise ValueError('case directory identity mismatch')
        expected = validate(path)
        if verify_receipts:
            actual = json.loads(path.with_name('validation.json').read_text())
            case, _ = load_case(path)
            for child in case['children']:
                expected['checks'] += [
                    {'child_id': child['id'], 'fixture': identity, 'exit_code': code}
                    for identity, code in [('gold', 0), ('nop', 1)]
                    + [(negative['id'], 1) for negative in child['negatives']]]
            if actual != expected:
                raise ValueError('missing, stale or invalid fixture receipt')
    return len(cases) * 4


def materialize(path, output):
    """Write four standard Harbor tasks; never copy an entire case directory."""
    path, output = Path(path), Path(output)
    case, _ = load_case(path)
    output.mkdir(parents=True, exist_ok=False)
    for child in case['children']:
        task = output / (case['parent_id'] + '-' + child['id'])
        environment = task / 'environment'
        environment.mkdir(parents=True)
        shutil.copyfile(source(path.parent, child['instruction']), task / 'instruction.md')
        copy_files(path.parent, child['fixtures'], environment / 'fixtures')
        dockerfile = (f"FROM {case['image']}\nWORKDIR /app\n"
                      'COPY fixtures/ /app/\n')
        (environment / 'Dockerfile').write_text(dockerfile)
        tests = task / 'tests'
        tests.mkdir()
        shutil.copyfile(source(path.parent, child['checker']), tests / 'check.py')
        copy_files(path.parent, child['oracle'], tests / 'oracle')
        (tests / 'test.sh').write_text(
            '#!/bin/sh\nmkdir -p /logs/verifier\n'
            'python3 -c ' + shlex.quote(CHECKER_BOOTSTRAP) + '\ncode=$?\n'
            'case "$code" in\n0) echo 1 > /logs/verifier/reward.txt ;;\n'
            '1) echo 0 > /logs/verifier/reward.txt ;;\n*) exit "$code" ;;\nesac\n')
        solution = task / 'solution'
        solution.mkdir()
        copy_files(path.parent, child['gold'], solution / 'gold')
        commands = ['#!/bin/sh', 'set -eu']
        for target in child['gold']:
            commands += ['mkdir -p ' + shlex.quote(str(PurePosixPath('/app') / target).rsplit('/', 1)[0]),
                         'cp ' + shlex.quote('/solution/gold/' + target) + ' ' + shlex.quote('/app/' + target)]
        (solution / 'solve.sh').write_text('\n'.join(commands) + '\n')
        for script in (tests / 'test.sh', solution / 'solve.sh'):
            script.chmod(0o755)
        # Suite scheduling must enforce this policy; Harbor owns the timeouts.
        config = ('schema_version = "1.1"\n[agent]\ntimeout_sec = 900\n'
                  '[verifier]\ntimeout_sec = 120\n[environment]\n'
                  'cpus = 1\nmemory_mb = 512\nstorage_mb = 1024\ngpus = 0\n'
                  'network_mode = "no-network"\n[metadata]\nkind = "diagnostic"\n'
                  f'parent_id = "{case["parent_id"]}"\nchild_id = "{child["id"]}"\n'
                  'policy_json = ' + json.dumps(json.dumps(POLICY, sort_keys=True)) + '\n')
        (task / 'task.toml').write_text(config)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    single = commands.add_parser('validate')
    single.add_argument('--case', required=True, type=Path)
    single.add_argument('--run-fixtures', action='store_true')
    all_cases = commands.add_parser('validate-all')
    all_cases.add_argument('--catalog', required=True, type=Path)
    all_cases.add_argument('--cases-root', required=True, type=Path)
    all_cases.add_argument('--verify-receipts', action='store_true')
    package = commands.add_parser('materialize')
    package.add_argument('--case', required=True, type=Path)
    package.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    if args.command == 'validate':
        receipt = validate(args.case, args.run_fixtures)
        if args.run_fixtures:
            args.case.with_name('validation.json').write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps(receipt, sort_keys=True))
    elif args.command == 'validate-all':
        print(f'Validated {validate_all(args.catalog, args.cases_root, args.verify_receipts)} diagnostics')
    else:
        materialize(args.case, args.output)


if __name__ == '__main__':
    main()
