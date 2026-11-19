"""Stage only the pinned DimCode Darwin arm64 payload for managed signing."""
import hashlib
from pathlib import Path
import sys

SOURCE_SHA256 = "0d6298cc8881b15ae8cdf1901b6f39674b74dbf5c14caedadfe509ab5ae114b0"


def apply(source, target):
    data = source.read_bytes()
    if hashlib.sha256(data).hexdigest() != SOURCE_SHA256:
        raise RuntimeError("DimCode upstream executable digest mismatch")

    # Preserve the upstream ACP implementation and the Bun bundle's byte offsets.
    with target.open("xb") as output:
        output.write(data)
    target.chmod(0o700)


if __name__ == "__main__":
    apply(*map(Path, sys.argv[1:]))
