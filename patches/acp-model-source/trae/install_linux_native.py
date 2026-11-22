"""Stage the pinned official TRAE entry for CLI-local use."""
import hashlib
import json
from pathlib import Path
import platform
import struct
import sys
import tarfile


def install(archive, runtime):
    contract = json.loads(Path(__file__).with_name("linux.json").read_text(encoding="utf-8"))
    target = contract["targets"].get(platform.machine())
    if platform.system() != "Linux" or target is None:
        raise ValueError("Unsupported TRAE Linux target")

    with archive.open("rb") as source:
        digest = hashlib.sha256()
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
        if digest.hexdigest() != target["archive_sha256"]:
            raise ValueError("TRAE Linux archive checksum mismatch")
        source.seek(0)
        with tarfile.open(fileobj=source, mode="r:gz") as package:
            members = package.getmembers()
            if len(members) != 1 or members[0].name != contract["archive_entry"] or not members[0].isfile():
                raise ValueError("Unexpected TRAE Linux archive layout")
            with package.extractfile(members[0]) as binary:
                executable = binary.read()

    if hashlib.sha256(executable).hexdigest() != target["executable_sha256"]:
        raise ValueError("TRAE Linux executable checksum mismatch")
    if (len(executable) < 64 or executable[:7] != b"\x7fELF\x02\x01\x01"
            or struct.unpack_from("<H", executable, 18)[0] != target["elf_machine"]):
        raise ValueError("TRAE Linux executable architecture mismatch")

    runtime.mkdir(mode=0o700)
    entry = runtime / contract["entry"]
    with entry.open("xb") as output:
        output.write(executable)
    entry.chmod(0o700)


if __name__ == "__main__":
    install(Path(sys.argv[1]), Path(sys.argv[2]))
