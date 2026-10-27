"""Black-box synthetic Git programs; all expectations come from fresh fixtures."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ENV = {**os.environ, 'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': '/dev/null'}


def git(repo, *args):
    return subprocess.check_output(['git', '-C', str(repo), *args], env=ENV,
                                   stderr=subprocess.DEVNULL)


def objects(repo):
    return [line.split() for line in git(repo, 'cat-file', '--batch-all-objects',
                                        '--batch-check=%(objectname) %(objecttype)').decode().splitlines()]


def commit(repo, name, value, message):
    (repo / name).write_text(value)
    git(repo, 'add', name)
    git(repo, 'commit', '-m', message)


def setup(repo, index, packed):
    repo.mkdir()
    git(repo, 'init', '-b', 'main')
    git(repo, 'config', 'user.name', 'Synthetic Fixture')
    git(repo, 'config', 'user.email', 'synthetic@example.invalid')
    git(repo, 'config', 'gc.auto', '0')
    git(repo, 'config', 'core.logAllRefUpdates', 'true')
    commit(repo, 'README.md', f'fixture {index}\n', 'Initial documentation')
    credential = f'secret[fictional_test_{index}_7e9c]'
    commit(repo, 'scratch.txt', credential + '\n', 'Temporary scratch notes')
    git(repo, 'reset', '--hard', 'HEAD~1')
    commit(repo, 'keep.txt', 'preserve this data\n', 'Keep legitimate work')
    # Independent orphan blob prevents dangling-commit-only enumeration shortcuts.
    subprocess.run(['git', '-C', str(repo), 'hash-object', '-w', '--stdin'],
                   input=b'independent harmless orphan\n', capture_output=True, env=ENV, check=True)
    if packed:
        # Reflog retention makes the lost credential enter a pack without pruning it.
        git(repo, 'gc', '--prune=never')
    git(repo, 'status', '--porcelain')
    spec = dict(credential=credential,
                files={'README.md': f'fixture {index}\n', 'keep.txt': 'preserve this data\n'},
                messages=['Keep legitimate work', 'Initial documentation'])
    return spec


def snapshot(root):
    return {str(p.relative_to(root)): p.read_bytes() for p in root.rglob('*') if p.is_file()}


def reachable(repo):
    return {line.split()[0] for line in git(repo, 'rev-list', '--objects', '--all').decode().splitlines()}


def retained(repo):
    return (git(repo, 'show-ref'), git(repo, 'log', '--all', '--format=%H %P %s'),
            {name: (repo / name).read_bytes() for name in ('README.md', 'keep.txt')})


def main():
    app, oracle = map(Path, sys.argv[1:])
    child = json.loads((oracle / 'task.json').read_text())['child']
    program = app / 'solution.py'
    if not program.is_file() or program.is_symlink():
        return 1
    variants = ['valid', 'leaked', 'wrong-recovery', 'missing-recovery', 'symlink-recovery',
                'dirty-file', 'changed-message', 'untracked'] if child == 'm4' else ['loose', 'packed']
    with tempfile.TemporaryDirectory(prefix='.git-leak-', dir=app) as scratch:
        scratch = Path(scratch)
        for index, variant in enumerate(variants):
            repo = scratch / str(index)
            spec = setup(repo, index, index % 2 == 1)
            recovered = scratch / f'recovered-{index}.txt'
            manifest = scratch / f'manifest-{index}.json'
            manifest.write_text(json.dumps(spec))
            args = [str(repo)]
            if child == 'm2':
                args += [str(recovered)]
            if child == 'm4':
                if variant != 'leaked':
                    git(repo, 'reflog', 'expire', '--expire=now', '--all')
                    git(repo, 'gc', '--prune=now')
                recovered.write_text(spec['credential'] + '\n')
                if variant == 'wrong-recovery':
                    recovered.write_text('secret[wrong]\n')
                elif variant == 'missing-recovery':
                    recovered.unlink()
                elif variant == 'symlink-recovery':
                    recovered.unlink()
                    recovered.symlink_to(manifest)
                elif variant == 'dirty-file':
                    (repo / 'keep.txt').write_text('incorrect\n')
                elif variant == 'changed-message':
                    git(repo, 'commit', '--amend', '-m', 'Changed legitimate message')
                elif variant == 'untracked':
                    (repo / 'extra.txt').write_text('extra\n')
                git(repo, 'status', '--porcelain')
                args += [str(recovered), str(manifest)]
                expected = dict(recovered_ok=variant not in ('wrong-recovery', 'missing-recovery', 'symlink-recovery'),
                                no_leak=variant != 'leaked',
                                preserved=variant not in ('dirty-file', 'changed-message', 'untracked'))
                expected['ready'] = all(expected.values())
            elif child == 'm1':
                refs = reachable(repo)
                expected = sorted([dict(oid=oid, type=kind) for oid, kind in objects(repo)
                                   if oid not in refs], key=lambda entry: entry['oid'])
            before = snapshot(repo)
            kept = retained(repo)
            external = snapshot(scratch) if child == 'm4' else None
            result = subprocess.run([sys.executable, str(program), *args], capture_output=True,
                                    env=ENV, timeout=20)
            if result.returncode:
                return 1
            if child in ('m1', 'm4'):
                try:
                    actual = json.loads(result.stdout)
                except (ValueError, UnicodeError):
                    return 1
                if json.dumps(actual, sort_keys=True) != json.dumps(expected, sort_keys=True):
                    return 1
            if child in ('m1', 'm2', 'm4') and snapshot(repo) != before:
                return 1
            if child == 'm2' and (not recovered.is_file() or recovered.is_symlink()
                                 or recovered.read_bytes() != (spec['credential'] + '\n').encode()):
                return 1
            if child == 'm3':
                try:
                    preserved = retained(repo) == kept and not git(repo, 'status', '--porcelain')
                except subprocess.CalledProcessError:
                    # Candidate destroyed repository history: a behavior rejection.
                    return 1
                if not preserved:
                    return 1
                if any(b'secret[' in git(repo, 'cat-file', '-p', oid) for oid, _ in objects(repo)):
                    return 1
            if child == 'm4' and snapshot(scratch) != external:
                return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
