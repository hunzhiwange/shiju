"""Stage the fixed JetBrains Linux CLI without invoking the upstream installer."""

import hashlib
import json
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import zipfile


def install(stage, runtime, arch):
    manifest = json.loads((stage / "linux-artifacts.json").read_text())
    target = manifest["targets"][arch]
    archive = stage / "junie.zip"
    subprocess.run(
        ["curl", "--disable", "--fail", "--silent", "--show-error", "--location",
         "--proto", "=https", "--proto-redir", "=https", "--max-time", "240",
         "--output", str(archive), target["archive_url"]],
        check=True,
    )
    checksum = hashlib.sha256()
    with archive.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(block)
    if checksum.hexdigest() != target["archive_sha256"]:
        raise ValueError("Junie Linux archive checksum mismatch")

    runtime.mkdir(mode=0o700)
    with zipfile.ZipFile(archive) as bundle:
        for item in bundle.infolist():
            if not item.filename.startswith("junie-app/") or item.is_dir():
                continue
            relative = Path(item.filename)
            mode = item.external_attr >> 16
            if (relative.as_posix() != item.filename or ".." in relative.parts
                    or not stat.S_ISREG(mode)):
                raise ValueError("Unexpected Junie Linux archive entry")
            destination = runtime / relative
            destination.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            with bundle.open(item) as source, destination.open("xb") as output:
                shutil.copyfileobj(source, output)
            destination.chmod(0o700 if mode & 0o111 else 0o600)
    for directory in runtime.rglob("*"):
        if directory.is_dir():
            directory.chmod(0o700)


if __name__ == "__main__":
    try:
        install(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3])
    except (OSError, ValueError, KeyError, zipfile.BadZipFile, subprocess.CalledProcessError):
        raise SystemExit("Junie Linux native installation failed") from None
