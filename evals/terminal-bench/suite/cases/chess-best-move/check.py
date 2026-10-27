"""Check independently specified synthetic chess artifacts, never final prose."""
import json
from pathlib import Path
import sys


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate JSON key')
        result[key] = value
    return result


def check(candidate, oracle):
    task = json.loads((oracle / 'task.json').read_text())
    output = candidate / task['output']
    if output.is_symlink() or not output.is_file() or output.stat().st_size > 16384:
        return False
    try:
        text = output.read_text(encoding='utf-8')
        if task['kind'] == 'm4':
            return text == task['expected']
        actual = json.loads(text, object_pairs_hook=unique_object)
        # Canonical JSON retains booleans as distinct from numeric 0/1.
        return json.dumps(actual, sort_keys=True) == json.dumps(task['expected'], sort_keys=True)
    except (OSError, UnicodeError, ValueError):
        return False


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]), Path(sys.argv[2])) else 1)
