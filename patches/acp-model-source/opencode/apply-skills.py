"""Patch a hash-pinned private Darwin-arm64 Bun executable without moving modules.

The reviewed scanner and skill file listing retain their original byte lengths,
preserving all module offsets and native code; re-sign the private copy.
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
    patched = bytearray(original)
    previous_end = 0
    for edit in sorted(contract["edits"], key=lambda item: item["offset"]):
        before, after = edit["before"].encode(), edit["after"].encode()
        start = edit["offset"]
        end = start + len(before)
        if start < previous_end or original[start:end] != before or len(after) > len(before):
            raise ValueError('OpenCode reviewed skill source anchor mismatch')
        patched[start:end] = after.ljust(len(before))
        previous_end = end
    path.write_bytes(patched)
    subprocess.run(['codesign', '--force', '--sign', '-', '--identifier',
                    'ai.opencode.shiju.skills', '--timestamp=none', str(path)], check=True)
    subprocess.run(['codesign', '--verify', '--strict', str(path)], check=True)
    if hashlib.sha256(path.read_bytes()).hexdigest() != contract['patched_sha256']:
        raise ValueError('OpenCode signed patched executable checksum mismatch')


if __name__ == '__main__':
    patch(Path(sys.argv[1]))
