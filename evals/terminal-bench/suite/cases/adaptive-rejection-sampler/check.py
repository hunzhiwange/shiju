"""Execute candidate R code against independent assertions, never self-test text."""
import json
from pathlib import Path
import secrets
import subprocess
import sys
import tempfile


def main():
    candidate, oracle = (Path(arg).resolve() for arg in sys.argv[1:])
    if not (candidate / 'answer.R').is_file():
        return 1
    marker = 'verified-' + secrets.token_hex(24)
    script = ('candidate <- ' + json.dumps(str(candidate)) + '\n'
              'source(' + json.dumps(str(oracle / 'check.R')) + ')\n'
              'cat(' + json.dumps(marker) + ')\n')
    with tempfile.TemporaryDirectory() as directory:
        runner = Path(directory) / 'verify.R'
        runner.write_text(script)
        try:
            result = subprocess.run(['Rscript', '--vanilla', str(runner)],
                                    cwd=candidate, capture_output=True, text=True, timeout=90)
        except subprocess.TimeoutExpired:
            return 1
    if result.returncode != 0 or not result.stdout.endswith(marker):
        print(result.stderr[-3000:], file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
