"""Execute synthetic candidate programs; 1 rejects, 2 is infrastructure error."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate JSON key')
        result[key] = value
    return result


def typed_equal(a, b):
    if type(a) is not type(b):
        return False
    if isinstance(a, list):
        return len(a) == len(b) and all(typed_equal(x, y) for x, y in zip(a, b))
    if isinstance(a, dict):
        return a.keys() == b.keys() and all(typed_equal(a[k], b[k]) for k in a)
    return a == b


def main():
    app, oracle = map(Path, sys.argv[1:])
    expected = json.loads((oracle / 'expected.json').read_text())
    program, fixture = app / 'solution.py', app / 'input.json'
    if any(p.is_symlink() or not p.is_file() for p in (program, fixture)):
        return 1
    if program.stat().st_size > 65536 or fixture.stat().st_size > 65536:
        return 1
    before = fixture.read_bytes()
    if hashlib.sha256(before).hexdigest() != expected['input_sha256']:
        return 1
    try:
        command = [sys.executable, '-I', str(program.resolve())]
        with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
            result = subprocess.run(command, input=before, stdout=stdout, stderr=stderr,
                                    cwd=app, timeout=5)
            if result.returncode != 0 or stdout.tell() > 65536:
                return 1
            stdout.seek(0)
            output = stdout.read()
        if fixture.is_symlink() or not fixture.is_file() or fixture.read_bytes() != before:
            return 1
        actual = json.loads(output, object_pairs_hook=unique)
        return 0 if typed_equal(actual, expected['expected']) else 1
    except (ValueError, OSError, subprocess.TimeoutExpired):
        return 1


if __name__ == '__main__':
    sys.exit(main())
