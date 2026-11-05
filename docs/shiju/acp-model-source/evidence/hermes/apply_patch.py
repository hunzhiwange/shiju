#!/usr/bin/env python3
"""Apply only the reviewed Hermes 0.19.0 patch to an explicitly chosen package root."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-dir", required=True, type=Path)
    args = parser.parse_args()
    root = args.package_dir.resolve(strict=True)
    metadata = json.loads(Path(__file__).with_name("patch.json").read_text())
    paths = metadata["files"]
    if any(Path(name).is_absolute() or ".." in Path(name).parts for name in paths):
        raise SystemExit("Hermes patch paths are invalid; no files changed")
    if any((root / name).resolve() != root / name for name in paths):
        raise SystemExit("Hermes package symlinks are unsupported; no files changed")
    patch = Path(__file__).with_name(metadata["patch"]).resolve()
    if digest(patch) != metadata["patch_sha256"]:
        raise SystemExit("Hermes patch identity mismatch; no files changed")
    if all(digest(root / name) == value["patched_sha256"] for name, value in paths.items()):
        print("Hermes 0.19.0 session-state patch already verified")
        return
    if any((root / name).is_symlink() or digest(root / name) != value["original_sha256"]
           for name, value in paths.items()):
        raise SystemExit("Hermes source identity mismatch; no files changed")
    command = ["patch", "--batch", "--forward", "-p1", "-i", str(patch)]
    subprocess.run([*command, "--dry-run"], cwd=root, check=True)
    subprocess.run(command, cwd=root, check=True)
    if not all(digest(root / name) == value["patched_sha256"] for name, value in paths.items()):
        raise SystemExit("Hermes patch verification failed; do not launch this package")
    print("Hermes 0.19.0 session-state patch verified")


if __name__ == "__main__":
    main()
