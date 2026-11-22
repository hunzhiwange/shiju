"""Stage the reviewed GNU Linux Droid executable for host publication."""

import argparse
import json
import os
from pathlib import Path
import platform
import subprocess
from tempfile import TemporaryDirectory

from apply import apply

MANIFEST = Path(__file__).with_name("linux.json")


def install(runtime: Path) -> Path:
    manifest = json.loads(MANIFEST.read_text())
    arch = platform.machine()
    target = manifest["targets"].get(arch)
    if platform.system() != "Linux" or target is None or not os.confstr("CS_GNU_LIBC_VERSION"):
        raise RuntimeError("Droid has no reviewed executable for this Linux target")

    with TemporaryDirectory(prefix=".droid-download-", dir=runtime.parent) as directory:
        scratch = Path(directory)
        supplier = scratch / "droid-original"
        subprocess.run(
            [
                "/usr/bin/curl", "--disable", "--fail", "--silent", "--show-error",
                "--location", "--proto", "=https", "--proto-redir", "=https",
                "--max-filesize", str(target["source_size"]), "--max-time", "240",
                "--output", str(supplier), target["artifact_url"],
            ],
            check=True,
            stdin=subprocess.DEVNULL,
        )
        if supplier.stat().st_size != target["source_size"]:
            raise ValueError("Droid Linux supplier executable size mismatch")
        reviewed = scratch / manifest["bin"]
        apply(supplier, reviewed, arch)

        # The caller owns a staging parent on the same filesystem as the final runtime.
        runtime.mkdir(mode=0o700)
        entry = runtime / manifest["bin"]
        try:
            os.link(reviewed, entry)
        except OSError:
            runtime.rmdir()
            raise
        return entry


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("runtime", type=Path, help="A fresh directory inside an installer staging root")
    install(parser.parse_args().runtime)
