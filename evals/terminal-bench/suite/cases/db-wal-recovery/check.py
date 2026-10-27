"""Check typed recovery artifacts and query a real standalone SQLite database."""
import hashlib
import json
from pathlib import Path
import sqlite3
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
    output = app / ('recovered.db' if contract['kind'] == 'm4' else 'result.json')
    if output.is_symlink() or not output.is_file() or output.stat().st_size > 4 * 1024 * 1024:
        return False
    try:
        if contract['kind'] == 'm4':
            # immutable=1 prevents a sidecar WAL from masking a stale main database.
            with sqlite3.connect(output.as_uri() + '?mode=ro&immutable=1', uri=True, timeout=1) as con:
                if con.execute('PRAGMA integrity_check').fetchall() != [('ok',)]:
                    return False
                if [row[1] for row in con.execute('PRAGMA table_info(readings)')] != ['id', 'name', 'value']:
                    return False
                actual = [dict(zip(('id', 'name', 'value'), row)) for row in
                          con.execute('SELECT id,name,value FROM readings ORDER BY id')]
        else:
            actual = json.loads(output.read_text(), object_pairs_hook=unique)
        return same(actual, contract['expected'])
    except (ValueError, OSError, sqlite3.Error):
        return False


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()) else 1)
