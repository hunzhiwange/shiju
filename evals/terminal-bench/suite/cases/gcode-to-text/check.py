"""Execute candidate code on independently specified synthetic G-code probes."""
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile

def matches(actual, expected):
    if isinstance(expected, str):
        return isinstance(actual, str) and actual == expected
    if isinstance(expected, list):
        return (isinstance(actual, list) and len(actual) == len(expected)
                and all(matches(a, e) for a, e in zip(actual, expected)))
    return (type(actual) in (int, float) and math.isfinite(actual)
            and actual == expected)


app, oracle = map(Path, sys.argv[1:])
program = app / 'solution.py'
if not program.is_file():
    sys.exit(1)
probes = json.loads((oracle / 'probes.json').read_text())
for probe in probes:
    with tempfile.TemporaryDirectory() as directory:
        source = Path(directory) / 'input.gcode'
        source.write_text(probe['input'])
        try:
            result = subprocess.run(['python3', str(program), str(source)],
                                    cwd=app, capture_output=True, timeout=10)
        except subprocess.TimeoutExpired:
            sys.exit(1)
    if result.returncode != 0 or len(result.stdout) > 100000:
        sys.exit(1)
    try:
        actual = (result.stdout.decode().strip() if isinstance(probe['expected'], str)
                  else json.loads(result.stdout))
    except (ValueError, UnicodeError):
        sys.exit(1)
    if not matches(actual, probe['expected']):
        sys.exit(1)
sys.exit(0)
