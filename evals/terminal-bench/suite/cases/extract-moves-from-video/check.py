"""Check exact synthetic transcription artifacts and unchanged public inputs."""
import hashlib
import json
from pathlib import Path
import sys


def check(app, oracle):
    contract = json.loads((oracle / 'expected.json').read_text())
    for name, expected_hash in contract['inputs'].items():
        path = app / name
        if (path.is_symlink() or not path.is_file()
                or hashlib.sha256(path.read_bytes()).hexdigest() != expected_hash):
            return False
    output = app / contract['output']
    if output.is_symlink() or not output.is_file() or output.stat().st_size > 1024 * 1024:
        return False
    try:
        data = output.read_bytes()
        if contract['format'] == 'text':
            return data == contract['expected'].encode('utf-8')
        actual = json.loads(data)
        return (type(actual) is list and all(type(item) is str for item in actual)
                and actual == contract['expected'])
    except (ValueError, OSError):
        return False


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()) else 1)
