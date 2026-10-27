"""Execute synthetic submissions; expected behavior comes from independent specs."""
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
            raise ValueError('duplicate key')
        result[key] = value
    return result


def decode(text):
    return json.loads(text, object_pairs_hook=unique)


def same(actual, expected):
    return json.dumps(actual, sort_keys=True, allow_nan=False) == json.dumps(expected, sort_keys=True)


def expected_digest(data):
    digest = hashlib.sha256(bytes(reversed(data))).digest()
    return hashlib.sha256(digest[2:9] + b'IMAGE-LOOP-47' + digest).hexdigest() + '\n'


def invoke(script, work, value=None, args=()):
    result = subprocess.run([sys.executable, str(script), *map(str, args)],
                            input=json.dumps(value) if value is not None else '',
                            text=True, capture_output=True, timeout=3, cwd=work,
                            env={'PATH': '/usr/local/bin:/usr/bin:/bin',
                                 'PYTHONDONTWRITEBYTECODE': '1'})
    if result.returncode or len(result.stdout) > 65536:
        raise ValueError('candidate execution failed')
    return result.stdout


def check(app, oracle):
    task = decode((oracle / 'task.json').read_text())
    kind = task['kind']
    if kind == 'm1':
        artifact = app / 'trace.json'
        return not artifact.is_symlink() and same(decode(artifact.read_text()), task['expected'])
    name = {'m2': 'fold.py', 'm3': 'scan.py', 'm4': 'emit.py'}[kind]
    script = app / name
    if script.is_symlink() or not script.is_file():
        return False
    source = script.read_bytes()
    if kind == 'm4':
        output = app / 'output.txt'
        if output.is_symlink() or output.read_text() != expected_digest((app / 'sample.bin').read_bytes()):
            return False
    probes = task['hex_inputs'] if kind == 'm4' else task['inputs']
    for value in probes:
        with tempfile.TemporaryDirectory(prefix='image-code-check-') as directory:
            work = Path(directory)
            candidate = work / 'candidate.py'
            candidate.write_bytes(source)
            if kind == 'm2':
                # Weighted sum is an independent closed form for the iterative fold.
                expected = ((value['seed'] + sum((i+1)*x for i,x in enumerate(value['values']))) % 257
                            if value['values'] else value['seed'])
                actual = decode(invoke(candidate, work, value))
            elif kind == 'm3':
                a = value['values']
                index = value['start']
                visited = []
                while index < len(a) and a[index] >= 0:
                    visited.append(index)
                    index += 1 + int(a[index] == 0)
                expected = {'visited': visited, 'total': sum(a[i] for i in visited), 'stop': index}
                actual = decode(invoke(candidate, work, value))
            else:
                data = bytes.fromhex(value)
                input_path, output_path = work / 'input bytes.bin', work / 'result value.txt'
                input_path.write_bytes(data)
                invoke(candidate, work, args=(input_path, output_path))
                if output_path.is_symlink() or input_path.read_bytes() != data:
                    return False
                actual, expected = output_path.read_text(), expected_digest(data)
            if not same(actual, expected):
                return False
    return True


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]), Path(sys.argv[2]))
    except (OSError, ValueError, TypeError, KeyError, subprocess.TimeoutExpired):
        passed = False
    sys.exit(0 if passed else 1)
