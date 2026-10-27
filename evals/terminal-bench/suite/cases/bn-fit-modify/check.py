"""Independent synthetic artifact checker. Invalid artifacts reject, bugs raise."""
import csv
import json
import math
from pathlib import Path
import sys


def read_csv(path, header):
    with path.open(newline='') as stream:
        rows = list(csv.reader(stream))
    if not rows or rows[0] != header or any(len(r) != len(header) for r in rows[1:]):
        raise ValueError('CSV schema mismatch')
    return rows[1:]


def same_json(actual, expected):
    if type(expected) in (int, float):
        return type(actual) in (int, float) and math.isfinite(actual) and abs(actual - expected) <= 1e-9
    if type(actual) is not type(expected):
        return False
    if isinstance(expected, dict):
        return actual.keys() == expected.keys() and all(same_json(actual[k], v) for k, v in expected.items())
    if isinstance(expected, list):
        return len(actual) == len(expected) and all(same_json(a, b) for a, b in zip(actual, expected))
    return actual == expected


def check(app, oracle):
    spec = json.loads((oracle / 'expected.json').read_text())
    try:
        if spec['kind'] == 'json':
            return same_json(json.loads((app / spec['output']).read_text()), spec['value'])
        edges = read_csv(app / spec['edges_file'], ['from', 'to'])
        if len(edges) != len(spec['edges']) or {tuple(e) for e in edges} != {tuple(e) for e in spec['edges']}:
            return False
        if spec['kind'] == 'samples':
            rows = read_csv(app / 'final_bn_sample.csv', spec['columns'])
            if len(rows) != len(spec['rows']):
                return False
            for actual, wanted in zip(rows, spec['rows']):
                for a, b in zip(actual, wanted):
                    value = float(a)
                    if not math.isfinite(value) or abs(value - b) > 1e-9:
                        return False
        return True
    except (OSError, ValueError, TypeError):
        return False


if __name__ == '__main__':
    sys.exit(0 if check(*map(Path, sys.argv[1:])) else 1)
