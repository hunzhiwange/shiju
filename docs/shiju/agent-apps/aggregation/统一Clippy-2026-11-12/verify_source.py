#!/usr/bin/env python3
"""Read-only verification that a successful unified Clippy still covers this source."""
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = Path(subprocess.check_output(['git', 'rev-parse', '--show-toplevel'], cwd=HERE, text=True).strip())
result = json.loads((HERE / 'result.json').read_text())
phase = next(stage for stage in result['stages'] if stage['name'] == 'clippy')
if phase.get('exit_code') != 0 or not phase.get('source_unchanged'):
    print('Not reusable: Clippy is unfinished, failed, or crossed a source change.')
    sys.exit(2)
gate = json.loads((HERE / 'acceptance.json').read_text())
if not gate.get('task41_fix_delivered'):
    print('Not reusable as final evidence: task41 delivery gate is pending.')
    sys.exit(2)
current_toolchain = subprocess.check_output(['rustc', '--version', '--verbose'], cwd=ROOT, text=True)
if current_toolchain != (HERE / 'toolchain.txt').read_text():
    print('Not reusable: Rust toolchain differs from the captured check.')
    sys.exit(1)
expected = phase['source_end']
with gzip.open(HERE / ('source-' + expected + '.json.gz'), 'rt', encoding='utf-8') as stream:
    manifest = json.load(stream)
delivered = {entry['path']: entry['sha256'] for entry in manifest['entries']}
for name, expected_hash in gate.get('source_hashes', {}).items():
    if delivered.get(name) != expected_hash:
        print('Not reusable: final Clippy does not cover task41 delivered source:', name)
        sys.exit(2)
names = subprocess.check_output(['git', 'ls-files', '-c', '-o', '--exclude-standard', '-z'], cwd=ROOT).decode().split('\0')
entries = []
for name in sorted(set(names)):
    if not name or name.startswith(('.plans/', 'docs/')):
        continue
    path = ROOT / name
    if path.is_file():
        data = path.read_bytes()
        entries.append({'path': name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
actual = hashlib.sha256(json.dumps(entries, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()
print('Expected:', expected)
print('Current: ', actual)
if actual != expected:
    old = {entry['path']: entry['sha256'] for entry in manifest['entries']}
    new = {entry['path']: entry['sha256'] for entry in entries}
    changed = sorted(name for name in old.keys() | new.keys() if old.get(name) != new.get(name))
    print('Changed source paths:', len(changed))
    for name in changed[:60]:
        print(name)
    sys.exit(1)
print('Reusable: successful unchanged-source Clippy matches this worktree; focused tests remain task-owned.')
