"""Patch a hash-pinned private Darwin-arm64 Bun executable without moving modules.

Only the reviewed native skill scanner/state span changes. Its byte length stays
fixed, preserving all module offsets and native code; re-sign the private copy.
"""
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import sys


def patch(path):
    contract = json.loads(Path(__file__).with_name('skill-patch.json').read_text())
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise ValueError('OpenCode reviewed skill runtime is Darwin-arm64 only')
    original = path.read_bytes()
    if hashlib.sha256(original).hexdigest() != contract['original_sha256']:
        raise ValueError('OpenCode original executable checksum mismatch')
    before, after = contract['before'].encode(), contract['after'].encode()
    start = contract['offset']
    if original[start:start + len(before)] != before or len(after) > len(before):
        raise ValueError('OpenCode reviewed skill source anchor mismatch')
    path.write_bytes(original[:start] + after + b' ' * (len(before) - len(after))
                     + original[start + len(before):])
    subprocess.run(['codesign', '--force', '--sign', '-', '--identifier',
                    'ai.opencode.shiju.skills', '--timestamp=none', str(path)], check=True)
    if hashlib.sha256(path.read_bytes()).hexdigest() != contract['patched_sha256']:
        raise ValueError('OpenCode signed patched executable checksum mismatch')


if __name__ == '__main__':
    patch(Path(sys.argv[1]))
