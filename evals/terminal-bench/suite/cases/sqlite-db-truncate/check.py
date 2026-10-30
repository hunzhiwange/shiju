"""Execute recovery against two synthetic databases and compare actual output."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def unique(pairs):
    result = {}
    for k, v in pairs:
        if k in result:
            raise ValueError('duplicate key')
        result[k] = v
    return result


def same(a, b):
    if type(a) is not type(b):
        return False
    if isinstance(b, dict):
        return a.keys() == b.keys() and all(same(a[k], v) for k, v in b.items())
    if isinstance(b, list):
        return len(a) == len(b) and all(same(x, y) for x, y in zip(a, b))
    return a == b


def check(app, oracle):
    task = json.loads((oracle / 'task.json').read_text())
    program = app / 'solution.py'
    source = app / 'trunc.db'
    if any(p.is_symlink() or not p.is_file() for p in (program, source)):
        return False
    if program.stat().st_size > 64000 or hashlib.sha256(source.read_bytes()).hexdigest() != task['input_sha256']:
        return False
    for db, expected in zip((source, oracle / 'probe.db'), task['expected']):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            input_db, output = work / 'input.db', work / 'result.json'
            original = db.read_bytes()
            input_db.write_bytes(original)
            try:
                result = subprocess.run([sys.executable, '-I', str(program.resolve()), str(input_db), str(output)], cwd=work,
                                        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=5)
                if result.returncode or input_db.read_bytes() != original:
                    return False
                if output.is_symlink() or not output.is_file() or output.stat().st_size > 100000:
                    return False
                actual = json.loads(output.read_text(), object_pairs_hook=unique,
                                    parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))
                if not same(actual, expected):
                    return False
            except (OSError, ValueError, subprocess.TimeoutExpired):
                return False
    return True


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]), Path(sys.argv[2])) else 1)
