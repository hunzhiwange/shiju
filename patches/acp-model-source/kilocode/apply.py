"""Patch only the hash-pinned Kilocode Bun entry source in a private macOS copy.

Repack the upstream module graph without rebuilding or replacing its agent, tools,
resources or native runtime. Every other embedded module remains byte-identical.
"""
import hashlib
import json
from pathlib import Path
import platform
import struct
import subprocess
import sys


def digest(data):
    return hashlib.sha256(data).hexdigest()


def patch(path):
    contract = json.loads(Path(__file__).with_name('patch.json').read_text())
    original = path.read_bytes()
    if platform.system() != 'Darwin' or platform.machine() != 'arm64':
        raise ValueError('Kilocode reviewed embedded source is Darwin-arm64 only')
    if digest(original) != contract['original_sha256']:
        raise ValueError('Kilocode original executable checksum mismatch')
    binary = bytearray(original)
    position = 32
    section = None
    for _ in range(struct.unpack_from('<I', binary, 16)[0]):
        command, length = struct.unpack_from('<II', binary, position)
        if command == 25 and binary[position + 8:position + 24].rstrip(b'\0') == b'__BUN':
            section = position + 72
            break
        position += length
    if section is None:
        raise ValueError('Kilocode Bun source section missing')
    base = struct.unpack_from('<I', binary, section + 48)[0] + 8
    size = struct.unpack_from('<Q', binary, base - 8)[0]
    graph = bytearray(binary[base:base + size])
    if graph[-16:] != b'\n---- Bun! ----\n':
        raise ValueError('Kilocode Bun graph trailer mismatch')
    footer = list(struct.unpack_from('<QIIIIII', graph, len(graph) - 48))
    modules, table_length = footer[1:3]
    if table_length % 52 or footer[3] != 0:
        raise ValueError('Kilocode Bun module table mismatch')
    source_offset, source_size = struct.unpack_from('<II', graph, modules + 8)
    if source_offset != 0:
        raise ValueError('Kilocode entry source offset mismatch')
    source = bytes(graph[:source_size])
    if digest(source) != contract['entry_sha256']:
        raise ValueError('Kilocode entry source checksum mismatch')
    text = source.decode('utf-16le')
    for before, after in contract['edits']:
        if text.count(before) != 1:
            raise ValueError('Kilocode reviewed source anchor mismatch')
        text = text.replace(before, after)
    payload = text.encode('utf-16le')
    if digest(payload) != contract['patched_entry_sha256']:
        raise ValueError('Kilocode patched entry source checksum mismatch')
    delta = len(payload) - source_size
    updated = bytearray(payload) + graph[source_size:]
    for index in range(table_length // 52):
        row = modules + delta + index * 52
        for field in range(6):
            offset, length = struct.unpack_from('<II', updated, row + field * 8)
            if offset >= source_size:
                offset += delta
            if index == 0 and field == 1:
                length += delta
            struct.pack_into('<II', updated, row + field * 8, offset, length)
        if index:
            old_offset, old_length = struct.unpack_from('<II', graph, modules + index * 52 + 8)
            new_offset, new_length = struct.unpack_from('<II', updated, row + 8)
            if graph[old_offset:old_offset + old_length] != updated[new_offset:new_offset + new_length]:
                raise ValueError('Kilocode non-entry module changed')
    footer[0] += delta
    footer[1] += delta
    footer[4] += delta
    struct.pack_into('<QIIIIII', updated, len(updated) - 48, *footer)
    # Use only existing zero padding in this Mach-O segment; never move native code.
    capacity = struct.unpack_from('<Q', binary, position + 48)[0] - 8
    if len(updated) > capacity or any(binary[base + size:base + capacity]):
        raise ValueError('Kilocode source exceeds verified segment padding')
    binary[base:base + capacity] = updated + b'\0' * (capacity - len(updated))
    struct.pack_into('<Q', binary, base - 8, len(updated))
    struct.pack_into('<Q', binary, section + 40, len(updated) + 8)
    path.write_bytes(binary)
    subprocess.run(['codesign', '--force', '--sign', '-', '--identifier',
                    'ai.kilo.cli.shiju.skills', '--timestamp=none', str(path)], check=True)
    if digest(path.read_bytes()) != contract['patched_sha256']:
        raise ValueError('Kilocode signed patched executable checksum mismatch')


if __name__ == '__main__':
    patch(Path(sys.argv[1]))
