"""Stage the fixed Corust Linux x64 binary for ordinary ACP launches."""

import hashlib
import json
import platform
import shutil
import tarfile
from pathlib import Path


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(block)
    return result.hexdigest()


def install_native(archive, staging):
    """Extract into a fresh, installer-owned staging directory for later publication."""
    if (platform.system(), platform.machine()) != ("Linux", "x86_64"):
        raise ValueError("Corust 0.6.0 has no reviewed archive for this Linux architecture")
    contract = json.loads(Path(__file__).with_name("linux-x64.json").read_text())
    if digest(archive) != contract["archive_sha256"]:
        raise ValueError("Corust Linux archive checksum mismatch")
    entry = contract["bin"]
    with tarfile.open(archive, "r:gz") as package:
        members = package.getmembers()
        if len(members) != 1 or members[0].name != entry or not members[0].isfile():
            raise ValueError("Unexpected Corust Linux archive layout")
        staging.mkdir(mode=0o700)
        binary = staging / entry
        with package.extractfile(members[0]) as source, binary.open("xb") as output:
            shutil.copyfileobj(source, output)
    if digest(binary) != contract["files"][entry]:
        raise ValueError("Corust Linux executable checksum mismatch")
    binary.chmod(0o700)
    return binary, contract
