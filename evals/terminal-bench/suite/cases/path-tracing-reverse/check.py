"""Compile the submitted source and compare real output with frozen synthetic vectors."""
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import zlib


def process(command, work):
    return subprocess.run(command, cwd=work, input='', text=True, capture_output=True,
                          timeout=10, env={'PATH': os.defpath})


def equal_tokens(output, expected):
    actual = output.split()
    if len(actual) != len(expected):
        return False
    for observed, wanted in zip(actual, expected):
        if wanted in ('P3', 'INVALID'):
            if observed != wanted:
                return False
        elif not re.fullmatch(r'[+-]?[0-9]+', observed) or int(observed) != int(wanted):
            return False
    return True


def check(app, oracle):
    samples = json.loads((oracle/'task.json').read_text())['samples']
    source = app/'reproduce.c'
    if source.is_symlink() or not source.is_file():
        return False
    data = source.read_bytes()
    if len(data) > 65536 or len(zlib.compress(data)) >= 2100:
        return False
    compiler = shutil.which('gcc' if sys.platform.startswith('linux') else 'cc')
    if compiler is None:
        raise RuntimeError('infrastructure: C compiler missing')
    for sample in samples:
        with tempfile.TemporaryDirectory(prefix='.reverse-check-', dir=app) as directory:
            work = Path(directory)
            candidate = work/'reproduce.c'
            binary = work/'reproduce'
            candidate.write_bytes(data)
            flags = ['-static'] if sys.platform.startswith('linux') else []
            compiled = process([compiler, *flags, '-std=c11', '-o', str(binary),
                                str(candidate), '-lm'], work)
            if compiled.returncode:
                return False
            candidate.unlink()
            actual = process([str(binary), *sample['argv']], work)
            if (actual.returncode != sample['exit'] or len(actual.stdout) > 262144
                    or not equal_tokens(actual.stdout, sample['tokens'])):
                return False
    return True


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]), Path(sys.argv[2]))
    except (OSError, ValueError, TypeError, subprocess.TimeoutExpired):
        passed = False
    sys.exit(0 if passed else 1)
