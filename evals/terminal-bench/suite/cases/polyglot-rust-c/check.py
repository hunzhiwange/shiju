"""Compile one unchanged source as Rust and C++; compare actual process results."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def run(command, work):
    environment = {'PATH': os.environ.get('PATH', os.defpath)}
    for key in ('RUSTUP_HOME', 'CARGO_HOME'):
        if key in os.environ:
            environment[key] = os.environ[key]
    return subprocess.run(command, cwd=work, capture_output=True, timeout=15,
                          env=environment)


def check(app, oracle):
    task = json.loads((oracle/'task.json').read_text())
    source = app/'shared.rs'
    if source.is_symlink() or not source.is_file() or source.stat().st_size > 65536:
        return False
    rustc, cpp = shutil.which('rustc'), shutil.which('g++')
    if not rustc or not cpp:
        raise RuntimeError('Rust/C++ compiler unavailable: infrastructure failure')
    with tempfile.TemporaryDirectory(prefix='.polyglot-check-', dir=app) as directory:
        work = Path(directory)
        candidate = work/'shared.rs'
        candidate.write_bytes(source.read_bytes())
        binaries = [work/'rust-entry', work/'cpp-entry']
        for command in ([rustc, str(candidate), '-o', str(binaries[0])],
                        [cpp, '-x', 'c++', str(candidate), '-o', str(binaries[1])]):
            if run(command, work).returncode:
                return False
        for args in task['inputs']:
            code = 0
            kind = task['kind']
            if kind == 'm1':
                expected = 'bridge-43\n'
            elif kind == 'm2':
                n = int(args[0])
                expected = f'{n*(n+1)*(2*n+1)//6}\n'
            elif kind == 'm3':
                valid = (len(args) == 1 and 1 <= len(args[0]) <= 6
                         and all('0' <= c <= '9' for c in args[0]))
                code = 0 if valid else 2
                expected = f'{int(args[0])*1000000007+23}\n' if valid else ''
            elif kind == 'm4':
                n = int(args[0])
                expected = f'{n*n+5*n+19} {(n*n+31)%103}\n'
            else:
                raise ValueError('unsupported child')
            for binary in binaries:
                actual = run([str(binary), *args], work)
                if actual.returncode != code or actual.stdout != expected.encode():
                    return False
    return True


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
    except (OSError, subprocess.TimeoutExpired):
        passed = False
    sys.exit(0 if passed else 1)
