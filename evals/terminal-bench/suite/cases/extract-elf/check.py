"""Validate typed memory extraction artifacts against independent synthetic contracts."""
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


def check(app, oracle):
    contract = json.loads((oracle / 'expected.json').read_text())
    for name, digest in contract['inputs'].items():
        path = app / name
        if path.is_symlink() or not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            return False
    output = app / 'result.json'
    if output.is_symlink() or not output.is_file() or output.stat().st_size > 1024 * 1024:
        return False
    try:
        actual = json.loads(output.read_text(), object_pairs_hook=unique)
        return same(actual, contract['expected'])
    except (ValueError, OSError):
        return False


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()) else 1)
