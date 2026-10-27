"""Independent artifact and executable checks; no parent tests or model text."""
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile


def run(args, cwd, input=None):
    return subprocess.run(args, cwd=cwd, input=input, capture_output=True, text=True, timeout=25)


def result(output):
    found = re.findall(r'^Results: (\d+) (\d+) (\d+)\s*$', output, re.M)
    return tuple(map(int, found[-1])) if found else None


def check(app, oracle):
    contract = json.loads((oracle / 'task.json').read_text())
    for name, expected in contract['immutable_fixtures'].items():
        path = app / name
        if path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            return False
    child = contract['child']
    if child == 'm1':
        data = (app / 'pmars_1.4.7.orig.tar.gz').read_bytes()
        sha = hashlib.sha256(data).hexdigest()
        dsc = (app / 'pmars.dsc').read_text()
        if f' {sha} {len(data)} pmars_1.4.7.orig.tar.gz' not in dsc:
            return False
        expected = {'source': 'pmars', 'upstream_version': '1.4.7',
                    'debian_version': '1.4.7-2', 'archive_sha256': sha}
        if json.loads((app / 'source.json').read_text()) != expected:
            return False
        with tarfile.open(app / 'pmars_1.4.7.orig.tar.gz') as archive:
            for member in archive.getmembers():
                target = app / member.name
                if member.isfile() and (not target.is_file() or target.is_symlink()
                                        or target.read_bytes() != archive.extractfile(member).read()):
                    return False
        return True
    if child == 'm2':
        # /tmp is intentionally mounted noexec by the container policy.
        with tempfile.TemporaryDirectory(prefix="pmars-check-", dir=app) as tmp:
            work = Path(tmp)
            shutil.copytree('/opt/pmars-0.9.2/src', work / 'src')
            src = work / 'src'
            shutil.copyfile(app / 'Makefile', src / 'Makefile')
            # Object files are never accepted as candidate evidence.
            for obj in src.glob('*.o'):
                obj.unlink()
            binary = src / 'pmars'
            binary.unlink(missing_ok=True)
            built = run(['make', '-j1'], src)
            if built.returncode or not binary.is_file():
                return False
            deps = run(['ldd', str(binary)], src)
            if deps.returncode or re.search(r'lib(X11|Xt|Xext|Xaw)', deps.stdout):
                return False
            (src / 'loop.red').write_text(';redcode-94\n;assert 1\nstart jmp start\nend start\n')
            (src / 'dead.red').write_text(';redcode-94\n;assert 1\ndat 0,0\nend\n')
            battle = run([str(binary), '-b', '-r', '3', '-f', 'loop.red', 'dead.red'], src)
            if battle.returncode or result(battle.stdout) != (3, 0, 0):
                return False
            debug = run([str(binary), '-e', 'loop.red'], src, 's\ns\ns\nq\n')
            # cdb q intentionally terminates the simulator with USERABORT (4).
            return debug.returncode == 4 and len(re.findall(r'\b\d{5}\s+\w+\.\w+', debug.stdout)) >= 2
    if child == 'm3':
        script = app / 'run_battle.py'
        if not script.is_file():
            return False
        for rounds in (1, 7, 13):
            for left, right in [('loop.red', 'dead.red'), ('dead.red', 'loop.red')]:
                args = [str(rounds), str(app / left), str(app / right)]
                actual = run([sys.executable, str(script), *args], app)
                expected = run(['/usr/local/bin/pmars', '-b', '-r', args[0], '-f', *args[1:]], app)
                if expected.returncode or actual.returncode != expected.returncode:
                    return False
                if actual.stdout != expected.stdout or actual.stderr != expected.stderr:
                    return False
        return True
    if child == 'm4':
        warrior = app / 'warrior.red'
        if not warrior.is_file():
            return False
        assembled = run(['/usr/local/bin/pmars', '-r', '0', str(warrior)], app)
        if assembled.returncode:
            return False
        lengths = re.findall(r'\(length (\d+)\)', assembled.stdout)
        if len(lengths) != 1 or not 1 <= int(lengths[0]) <= 3:
            return False
        for rounds in (3, 11):
            for reverse in (False, True):
                pair = [str(warrior), str(app / 'opponent.red')]
                if reverse:
                    pair.reverse()
                battle = run(['/usr/local/bin/pmars', '-b', '-f', '-r', str(rounds), *pair], app)
                expected = (0, rounds, 0) if reverse else (rounds, 0, 0)
                if battle.returncode or result(battle.stdout) != expected:
                    return False
        return True
    raise ValueError('unknown diagnostic')


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]), Path(sys.argv[2]))
    except (FileNotFoundError, ValueError, OSError, subprocess.TimeoutExpired):
        passed = False
    sys.exit(0 if passed else 1)
