"""Check complete synthetic sequence artifacts against independent frozen oracles."""
import hashlib
import json
from pathlib import Path
import sys


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate JSON key')
        result[key] = value
    return result


def same(actual, expected):
    if type(actual) is not type(expected):
        return False
    if isinstance(expected, dict):
        return actual.keys() == expected.keys() and all(same(actual[k], v) for k, v in expected.items())
    if isinstance(expected, list):
        return len(actual) == len(expected) and all(same(a, b) for a, b in zip(actual, expected))
    return actual == expected


def regular(path):
    return not path.is_symlink() and path.is_file() and path.stat().st_size <= 16384


def check(candidate, oracle):
    try:
        task = json.loads((oracle / 'expected.json').read_text())
        input_path, result_path = candidate / 'input.json', candidate / 'result.json'
        if not all(regular(p) for p in (input_path, result_path)):
            return False
        if hashlib.sha256(input_path.read_bytes()).hexdigest() != task['inputs']['input.json']:
            return False
        actual = json.loads(result_path.read_text(), object_pairs_hook=unique)
        if not same(actual, task['expected']):
            return False
        return True
    except (ValueError, KeyError, TypeError, OSError):
        return False


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]), Path(sys.argv[2])) else 1)
