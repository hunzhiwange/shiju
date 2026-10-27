"""Execute submitted programs against fresh synthetic Git objects and histories."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def git(repo, *args):
    return subprocess.check_output(['git', '-C', str(repo), *args], stderr=subprocess.DEVNULL).decode().strip()


def ancestor(repo, left, right):
    result = subprocess.run(['git', '-C', str(repo), 'merge-base', '--is-ancestor', left, right], capture_output=True)
    if result.returncode not in (0, 1):
        raise RuntimeError('invalid ancestry probe')
    return result.returncode == 0


def commit(repo, name, content, subject):
    (repo / name).write_text(content)
    git(repo, 'add', name)
    git(repo, 'commit', '-m', subject)
    return git(repo, 'rev-parse', 'HEAD')


def setup(repo, child):
    repo.mkdir()
    git(repo, 'init', '-b', 'master')
    git(repo, 'config', 'user.email', 'fixture@example.invalid')
    git(repo, 'config', 'user.name', 'Fixture User')
    base = commit(repo, 'about.txt', 'profile: initial\n', 'Initial profile')
    if child in ('m1', 'm2'):
        git(repo, 'checkout', '--detach', base)
        lost = commit(repo, 'about.txt', 'profile: recovered\n', 'Recover profile')
        git(repo, 'checkout', 'master')
        master = commit(repo, 'keep.txt', 'keep: master\n', 'Keep master work')
        git(repo, 'branch', 'topic', base)
        return dict(master=master, topic=base, lost=lost, lost_parents=[base], lost_on_master=False)
    if child == 'm3':
        commit(repo, 'status.txt', 'state: initial\n', 'Initial state')
        git(repo, 'branch', 'incoming')
        commit(repo, 'status.txt', 'state: master\n', 'Master state')
        master = commit(repo, 'master-only.txt', 'master only\n', 'Master file')
        git(repo, 'checkout', 'incoming')
        commit(repo, 'status.txt', 'state: incoming\n', 'Incoming state')
        incoming = commit(repo, 'incoming-only.txt', 'incoming only\n', 'Incoming file')
        git(repo, 'checkout', 'master')
        return dict(master=master, incoming=incoming)
    ready = commit(repo, 'profile.txt', 'profile: ready\n', 'Ready profile')
    git(repo, 'tag', 'recovered-tip', ready)
    return dict(base=base, ready=ready)


def snapshot(repo):
    return {str(p.relative_to(repo)): p.read_bytes() for p in repo.rglob('*') if p.is_file()}


def run(program, repo):
    result = subprocess.run([sys.executable, str(program), str(repo)], capture_output=True,
                            timeout=15, env={**os.environ, 'GIT_CONFIG_NOSYSTEM': '1',
                                             'GIT_CONFIG_GLOBAL': '/dev/null'})
    if result.returncode:
        return None
    return result.stdout


def audit_expected(repo):
    branch = subprocess.run(['git', '-C', str(repo), 'symbolic-ref', '--short', 'HEAD'], capture_output=True)
    head_file = subprocess.run(['git', '-C', str(repo), 'show', 'HEAD:profile.txt'], capture_output=True)
    expected = dict(on_master=branch.returncode == 0 and branch.stdout == b'master\n',
                    clean=git(repo, 'status', '--porcelain') == '',
                    content_ok=(repo / 'profile.txt').is_file()
                    and (repo / 'profile.txt').read_bytes() == b'profile: ready\n'
                    and head_file.returncode == 0 and head_file.stdout == b'profile: ready\n',
                    recovered=ancestor(repo, 'recovered-tip', 'master'))
    expected['ready'] = all(expected.values())
    return expected


def main():
    app, oracle = map(Path, sys.argv[1:])
    child = json.loads((oracle / 'task.json').read_text())['child']
    program = app / 'solution.py'
    if not program.is_file() or program.is_symlink():
        return 1
    # Git requires executable scratch space. /tmp is noexec in calibration.
    with tempfile.TemporaryDirectory(prefix='.git-probe-', dir=app) as scratch:
        variants = ['valid', 'detached', 'other-branch', 'untracked', 'dirty',
                    'wrong-committed', 'missing', 'not-recovered'] if child == 'm4' else ['default']
        for index, variant in enumerate(variants):
            repo = Path(scratch) / str(index)
            state = setup(repo, child)
            if variant == 'detached':
                git(repo, 'checkout', '--detach')
            elif variant == 'other-branch':
                git(repo, 'checkout', '-b', 'preview')
            elif variant == 'untracked':
                (repo / 'extra.txt').write_text('untracked\n')
            elif variant == 'dirty':
                (repo / 'profile.txt').write_text('profile: dirty\n')
            elif variant == 'wrong-committed':
                commit(repo, 'profile.txt', 'profile: wrong\n', 'Wrong profile')
            elif variant == 'missing':
                git(repo, 'rm', 'profile.txt')
                git(repo, 'commit', '-m', 'Remove profile')
            elif variant == 'not-recovered':
                git(repo, 'reset', '--hard', state['base'])
                commit(repo, 'profile.txt', 'profile: ready\n', 'Copy profile')
            expected = audit_expected(repo) if child == 'm4' else state
            before = snapshot(repo) if child in ('m1', 'm4') else None
            output = run(program, repo)
            if output is None:
                return 1
            if child in ('m1', 'm4'):
                try:
                    actual = json.loads(output)
                except (ValueError, UnicodeError):
                    return 1
                # Strict JSON types distinguish booleans from integers.
                if json.dumps(actual, sort_keys=True) != json.dumps(expected, sort_keys=True):
                    return 1
                if snapshot(repo) != before:
                    return 1
            else:
                if git(repo, 'symbolic-ref', '--short', 'HEAD') != 'master' or git(repo, 'status', '--porcelain'):
                    return 1
                if child == 'm2':
                    if not ancestor(repo, state['master'], 'master') or not ancestor(repo, state['lost'], 'master'):
                        return 1
                    wanted = {'about.txt': b'profile: recovered\n', 'keep.txt': b'keep: master\n'}
                else:
                    if git(repo, 'rev-list', '--parents', '-n', '1', 'HEAD').split()[1:] != [state['master'], state['incoming']]:
                        return 1
                    if (repo / '.git' / 'MERGE_HEAD').exists() or git(repo, 'ls-files', '-u'):
                        return 1
                    wanted = {'status.txt': b'state: combined\n', 'master-only.txt': b'master only\n', 'incoming-only.txt': b'incoming only\n'}
                if any(not (repo / name).is_file() or (repo / name).read_bytes() != value for name, value in wanted.items()):
                    return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
