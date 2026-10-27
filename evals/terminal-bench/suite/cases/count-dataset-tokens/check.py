"""Check real JSON artifacts against frozen synthetic Qwen token counts."""
import hashlib
import json
from pathlib import Path
import sys


def unique(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError('duplicate JSON key')
        value[key] = item
    return value


def check(candidate, oracle):
    task = json.loads((oracle / 'task.json').read_text())
    try:
        for name, expected in task['input_sha256'].items():
            path = candidate / name
            if path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
                return False
        output = candidate / 'result.json'
        if output.is_symlink() or not output.is_file() or output.stat().st_size > 16384:
            return False
        actual = json.loads(output.read_text(), object_pairs_hook=unique)
        # Canonical JSON preserves integer/bool/string distinctions and rejects
        # extra fields, reordered records, or plausible but inconsistent totals.
        return json.dumps(actual, sort_keys=True, allow_nan=False) == json.dumps(
            task['expected'], sort_keys=True, allow_nan=False)
    except (OSError, ValueError, UnicodeError):
        return False


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]), Path(sys.argv[2])) else 1)
