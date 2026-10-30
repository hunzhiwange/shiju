"""Compile and execute the same synthetic submission through both real parsers."""
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def run(command, work):
    return subprocess.run(command, cwd=work, capture_output=True, timeout=5,
                          env={'PATH': os.defpath, 'PYTHONDONTWRITEBYTECODE': '1'})


def check(app, oracle):
    task = json.loads((oracle/'task.json').read_text())
    source = app/'shared.py.c'
    if source.is_symlink() or not source.is_file() or source.stat().st_size > 65536:
        return False
    compiler = shutil.which('gcc' if sys.platform.startswith('linux') else 'cc')
    if compiler is None:
        raise RuntimeError('C compiler unavailable')
    with tempfile.TemporaryDirectory(prefix='.polyglot-check-', dir=app) as directory:
        work = Path(directory)
        candidate, binary = work/'shared.py.c', work/'compiled'
        candidate.write_bytes(source.read_bytes())
        compiled = run([compiler, str(candidate), '-o', str(binary)], work)
        if compiled.returncode:
            return False
        for args in task['inputs']:
            kind = task['kind']
            code = 0
            if kind == 'm1':
                expected = 'bridge-29\n'
            elif kind == 'm2':
                n = int(args[0])
                expected = str(n*(n+1)//2)+'\n'
            elif kind == 'm3':
                valid = (len(args) == 1 and bool(args[0])
                         and all('0' <= c <= '9' for c in args[0]) and int(args[0]) <= 20)
                code = 0 if valid else 2
                expected = str(math.factorial(int(args[0])))+'\n' if valid else ''
            elif kind == 'm4':
                n = int(args[0])
                expected = f'{n*n-3*n+7} {(17*n+11)%97}\n'
            else:
                raise ValueError('unsupported child')
            for command in ([sys.executable, str(candidate), *args], [str(binary), *args]):
                actual = run(command, work)
                if actual.returncode != code or actual.stdout != expected.encode():
                    return False
    return True


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
    except (OSError, ValueError, TypeError, KeyError, subprocess.TimeoutExpired):
        passed = False
    sys.exit(0 if passed else 1)
