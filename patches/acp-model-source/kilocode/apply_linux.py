"""Port the reviewed 7.8.18 skill scanner to the two pinned GNU ELF releases."""
import hashlib
import json
import mmap
import os
from pathlib import Path
from shutil import copyfile
import struct
import sys
import tempfile


def patch_graph(binary, target):
    # Offsets belong to the pinned executable. The loader's writable PT_LOAD
    # already covers this zero padding. No program header or native code moves.
    section = target["section_header"]
    start, size = struct.unpack_from("<QQ", binary, section + 24)
    base = start + 8
    end = start + size
    capacity = target["padding_end"]
    if (binary[:6] != b"\x7fELF\x02\x01"
            or struct.unpack_from("<H", binary, 18)[0] != target["machine"]
            or struct.unpack_from("<Q", binary, start)[0] != size - 8
            or binary[end - 16:end] != b"\n---- Bun! ----\n"
            or not end < capacity <= len(binary) or any(binary[end:capacity])):
        raise ValueError("unexpected Kilocode ELF graph layout")

    footer = list(struct.unpack_from("<QIIIIII", binary, end - 48))
    modules, table_size = footer[1:3]
    if (footer[0] != size - 56 or not table_size or table_size % 52 or footer[3] != 0
            or not 0 < modules < modules + table_size <= footer[0]):
        raise ValueError("unexpected Kilocode Bun module table")
    source_offset, source_size = struct.unpack_from("<II", binary, base + modules + 8)
    if source_offset != 0 or not 0 < source_size <= modules:
        raise ValueError("unexpected Kilocode Bun entry offset")
    source = binary[base:base + source_size]
    if hashlib.sha256(source).hexdigest() != target["entry_sha256"]:
        raise ValueError("Kilocode entry source checksum mismatch")
    for old, new in target["edits"]:
        old, new = old.encode("utf-16le"), new.encode("utf-16le")
        if source.count(old) != 1:
            raise ValueError("Kilocode reviewed source anchor mismatch")
        source = source.replace(old, new)

    delta = len(source) - source_size
    if not 0 < delta <= capacity - end:
        raise ValueError("Kilocode source exceeds verified ELF padding")
    if hashlib.sha256(source).hexdigest() != target["patched_entry_sha256"]:
        raise ValueError("Kilocode patched entry source checksum mismatch")

    binary.move(base + len(source), base + source_size, end - base - source_size)
    binary[base:base + len(source)] = source
    for index in range(table_size // 52):
        row = base + modules + delta + index * 52
        for field in range(6):
            offset, length = struct.unpack_from("<II", binary, row + field * 8)
            if offset >= source_size:
                offset += delta
            if index == 0 and field == 1:
                length += delta
            struct.pack_into("<II", binary, row + field * 8, offset, length)

    footer[0] += delta
    footer[1] += delta
    footer[4] += delta
    struct.pack_into("<QIIIIII", binary, end + delta - 48, *footer)
    struct.pack_into("<Q", binary, start, size - 8 + delta)
    struct.pack_into("<Q", binary, section + 32, size + delta)


def patch(source, output, arch):
    target = json.loads(Path(__file__).with_name("linux.json").read_text())["targets"][arch]
    if source.resolve(strict=True) == output.resolve():
        raise ValueError("Kilocode ELF patch requires a separate reviewed destination")
    with tempfile.NamedTemporaryFile(dir=output.parent, delete=False) as stream:
        staging = Path(stream.name)
    try:
        copyfile(source, staging)
        with staging.open("r+b") as stream, mmap.mmap(stream.fileno(), 0) as binary:
            if hashlib.sha256(binary).hexdigest() != target["original_sha256"]:
                raise ValueError("Kilocode original executable checksum mismatch")
            patch_graph(binary, target)
            if hashlib.sha256(binary).hexdigest() != target["patched_sha256"]:
                raise ValueError("Kilocode patched executable checksum mismatch")
        staging.chmod(0o700)
        os.replace(staging, output)
    finally:
        staging.unlink(missing_ok=True)


if __name__ == "__main__":
    arch, source, output = sys.argv[1:]
    patch(Path(source), Path(output), arch)
