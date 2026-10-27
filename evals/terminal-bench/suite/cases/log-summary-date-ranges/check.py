"""Compare the actual CSV artifact to independently frozen synthetic rows."""
import csv
import json
from pathlib import Path
import sys


def check(candidate, oracle):
    expected = json.loads((oracle / 'expected.json').read_text())['rows']
    try:
        with (candidate / 'summary.csv').open(newline='', encoding='utf-8') as stream:
            actual = list(csv.reader(stream, strict=True))
    except (OSError, UnicodeError, csv.Error):
        return False
    return actual == expected


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]), Path(sys.argv[2])) else 1)
