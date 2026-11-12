#!/usr/bin/env python3
"""Verify applicability of the real round4 pass, allowing only the observed later MCP51 test addition.

Exit 0 is scoped applicability, not a claim that new MCP51 tests were checked by round4.
"""
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = Path(subprocess.check_output(['git', 'rev-parse', '--show-toplevel'], cwd=HERE, text=True).strip())
result = json.loads((HERE / 'round4-result.json').read_text())
phase = next(stage for stage in result['stages'] if stage['name'] == 'clippy')
if phase.get('exit_code') != 0 or not phase.get('source_unchanged'):
    sys.exit('No successful stable historical check.')
if subprocess.check_output(['rustc', '--version', '--verbose'], cwd=ROOT, text=True) != (HERE / 'toolchain.txt').read_text():
    sys.exit('Rust toolchain changed.')
with gzip.open(HERE / ('source-' + phase['source_end'] + '.json.gz'), 'rt', encoding='utf-8') as stream:
    old = {entry['path']: entry['sha256'] for entry in json.load(stream)['entries']}
names = subprocess.check_output(['git', 'ls-files', '-c', '-o', '--exclude-standard', '-z'], cwd=ROOT).decode().split('\0')
new = {}
for name in sorted(set(names)):
    if not name or name.startswith(('.plans/', 'docs/')):
        continue
    path = ROOT / name
    if path.is_file():
        new[name] = hashlib.sha256(path.read_bytes()).hexdigest()
changed = sorted(name for name in old.keys() | new.keys() if old.get(name) != new.get(name))
module = 'crates/vw-agent/src/agent_apps/mcp/mod.rs'
tests = 'crates/vw-agent/src/agent_apps/mcp/security_tests.rs'
if set(changed) - {module, tests}:
    print('Other source changes; original tasks must re-evaluate:', *changed, sep='\n')
    sys.exit(1)
if changed:
    block = b'\n#[cfg(all(test, target_os = "macos"))]\nmod security_tests;\n'
    content = (ROOT / module).read_bytes()
    if content.count(block) != 1 or hashlib.sha256(content.replace(block, b'', 1)).hexdigest() != old.get(module):
        sys.exit('MCP module change is more than the exact test-only registration.')
gate = json.loads((HERE / 'acceptance.json').read_text())
if not gate.get('task41_fix_delivered'):
    sys.exit('Task41 delivery is unverified.')
for name, digest in gate['source_hashes'].items():
    if old.get(name) != digest or new.get(name) != digest:
        sys.exit('Task41 delivered source changed: ' + name)
print('Historical full Clippy: exit 0, stable snapshot ' + phase['source_end'])
print('Current production source/dependencies and task41 delivery still match this plan snapshot.')
print('Subsequent test-only paths:', ', '.join(changed) or 'none')
print('Scope check: exit 0. New MCP51 test code is NOT covered by round4; each original task owns its applicability decision.')
