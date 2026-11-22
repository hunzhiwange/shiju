"""Build the reviewed OpenCode 1.19.5 GNU/Linux ELF from a fixed native npm package.

Fixed-length edits change only embedded JavaScript. Padding preserves Bun
module offsets. The output is published after
both the source and the complete patched artifact match the review manifest.
"""

import argparse
import hashlib
import json
import mmap
import os
from pathlib import Path
import platform
import shutil
import tempfile


def digest(file):
    file.seek(0)
    sha256 = hashlib.sha256()
    for chunk in iter(lambda: file.read(1024 * 1024), b""):
        sha256.update(chunk)
    return sha256.hexdigest()


def apply(source, output, target):
    contract = json.loads(Path(__file__).with_name("linux.json").read_text())
    review = contract["targets"][target]
    if source.resolve(strict=True) == output.resolve():
        raise ValueError("OpenCode output must be separate from the native package")

    with tempfile.NamedTemporaryFile(dir=output.parent, prefix=".opencode-") as artifact:
        with source.open("rb") as original:
            shutil.copyfileobj(original, artifact)
        artifact.flush()
        if digest(artifact) != review["original_sha256"]:
            raise ValueError("OpenCode original executable checksum mismatch")

        with mmap.mmap(artifact.fileno(), 0) as binary:
            previous_end = 0
            for edit in review["edits"]:
                start = edit["offset"]
                before, after = edit["before"].encode(), edit["after"].encode()
                end = start + len(before)
                if (start < previous_end or end > len(binary)
                        or binary[start:end] != before or len(after) > len(before)):
                    raise ValueError("OpenCode reviewed Linux source anchor mismatch")
                binary[start:end] = after.ljust(len(before))
                previous_end = end
            binary.flush()
        if digest(artifact) != review["patched_sha256"]:
            raise ValueError("OpenCode reviewed Linux executable checksum mismatch")

        os.fchmod(artifact.fileno(), 0o755)
        # Keep the temporary file automatically cleaned up on validation failure.
        os.link(artifact.name, output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    target = {"x86_64": "x64", "aarch64": "arm64"}.get(platform.machine())
    if platform.system() != "Linux" or target is None:
        parser.error("OpenCode requires Linux GNU x64 or ARM64")
    apply(args.source, args.output, target)


if __name__ == "__main__":
    main()
