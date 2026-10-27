"""Independent numeric validation for synthetic probability artifacts."""
from pathlib import Path
import json
import math
import sys


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate key')
        result[key] = value
    return result


def number(value):
    return type(value) in (int, float) and math.isfinite(value)


def close(actual, expected, tolerance=1e-8):
    return number(actual) and abs(actual - expected) <= tolerance


def metrics(p):
    n = len(p)
    return {'forward': math.fsum(x * math.log(x * n) for x in p if x),
            'backward': 'infinity' if 0 in p else math.fsum(-math.log(x * n) for x in p) / n}


def metric_match(actual, expected):
    return (isinstance(actual, dict) and set(actual) == set(expected)
            and all(actual[k] == v if v == 'infinity' else close(actual[k], v)
                    for k, v in expected.items()))


def check(candidate, oracle):
    try:
        task = json.loads((oracle / 'task.json').read_text())
        path = candidate / 'result.json'
        if path.is_symlink() or not path.is_file() or path.stat().st_size > 16384:
            return False
        if json.loads((candidate / 'input.json').read_text()) != task['input']:
            return False
        actual = json.loads(path.read_text(), object_pairs_hook=unique)
        data, kind = task['input'], task['kind']
        if kind == 'm1':
            return metric_match(actual, metrics(data['probabilities']))
        if kind == 'm3':
            return (isinstance(actual, list) and len(actual) == len(data['distributions'])
                    and all(metric_match(a, metrics(p)) for a, p in zip(actual, data['distributions'])))
        if not isinstance(actual, dict):
            return False
        if kind == 'm4':
            if (set(actual) != {'counts', 'probabilities'} or actual['counts'] != data['counts']
                    or any(type(x) is not int for x in actual['counts'])):
                return False
            if not isinstance(actual['probabilities'], list) or len(actual['probabilities']) != len(data['counts']):
                return False
            p = [v for n, v in zip(data['counts'], actual['probabilities']) for _ in range(n)]
        else:
            if set(actual) != {'probabilities'}:
                return False
            p = actual['probabilities']
        return (isinstance(p, list) and len(p) == (data['size'] if kind == 'm2' else sum(data['counts']))
                and all(number(x) and 0 < x <= 1 for x in p)
                and abs(math.fsum(p) - 1) <= 1e-10
                and metric_match(metrics(p), data['targets']))
    except (ValueError, TypeError, KeyError, OSError, OverflowError):
        return False


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]), Path(sys.argv[2])) else 1)
