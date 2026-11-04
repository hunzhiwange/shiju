#!/usr/bin/env python3
"""Apply reviewed model routing changes to the exact Qwen 0.26.7 archive."""
import hashlib
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
patch = json.loads(Path(__file__).with_name('patch.json').read_text())
package = json.loads((root / 'package.json').read_text())
if (package['name'], package['version']) != ('@qwen-code/qwen-code', patch['version']):
    raise SystemExit('Qwen package identity mismatch')
updates = []
for row in patch['files']:
    path = root / row['path']
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != row['sha256']:
        raise SystemExit('Qwen source checksum mismatch: ' + row['path'])
    text = raw.decode()
    for before, after in row['replacements']:
        if text.count(before) != 1:
            raise SystemExit('Qwen patch context mismatch: ' + row['path'])
        text = text.replace(before, after)
    updates.append((path, text))
for path, text in updates:
    path.write_text(text)
print('Patched fixed Qwen 0.26.7')
