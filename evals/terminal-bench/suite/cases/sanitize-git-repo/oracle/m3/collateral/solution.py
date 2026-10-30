import hashlib
import json
from pathlib import Path
import re
import sys

KINDS = {'AWS_ID': '<your-aws-access-key-id>',
         'AWS_SECRET': '<your-aws-secret-access-key>',
         'GITHUB': '<your-github-token>', 'HF': '<your-huggingface-token>'}
PATTERN = re.compile(r'(?<![A-Za-z0-9_])SYNTH_(AWS_ID|AWS_SECRET|GITHUB|HF)_[A-Z0-9]{8}(?![A-Za-z0-9_])')

def files(root):
    return sorted(p for p in root.rglob('*') if p.is_file() and '.git' not in p.relative_to(root).parts)

def scan(root):
    found = []
    for p in files(root):
        try:
            lines = p.read_text().splitlines()
        except UnicodeError:
            continue
        for line_no, line in enumerate(lines, 1):
            for m in PATTERN.finditer(line):
                found.append({'path': p.relative_to(root).as_posix(), 'line': line_no, 'kind': m[1]})
    return sorted(found, key=lambda x: (x['path'], x['line'], x['kind']))

def clean(p):
    try:
        value = p.read_bytes().decode('utf-8')
    except UnicodeError:
        return
    result = PATTERN.sub(lambda m: KINDS[m[1]], value)
    if result != value:
        p.write_bytes(result.encode('utf-8'))

for p in files(Path(sys.argv[1])):
    clean(p)
(Path(sys.argv[1])/'README.md').write_text('sanitized\n')
