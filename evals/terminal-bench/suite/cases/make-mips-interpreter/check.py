"""Execute the submitted stdin/stdout API against verifier-only synthetic probes."""
import json
from pathlib import Path
import subprocess
import sys


def main(app, oracle):
    candidate = app / 'run.py'
    if not candidate.is_file():
        return 1
    for probe in json.loads((oracle / 'probes.json').read_text()):
        try:
            result = subprocess.run([sys.executable, str(candidate)],
                                    input=json.dumps(probe['input']), text=True,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                    cwd=app, timeout=3)
        except subprocess.TimeoutExpired:
            return 1
        if result.returncode:
            return 1
        try:
            actual = json.loads(result.stdout)
        except (ValueError, UnicodeError):
            return 1
        # JSON's bool/int equality must not accept booleans as register values.
        if json.dumps(actual, sort_keys=True) != json.dumps(probe['output'], sort_keys=True):
            return 1
    return 0


if __name__ == '__main__':
    sys.exit(main(Path(sys.argv[1]), Path(sys.argv[2])))
