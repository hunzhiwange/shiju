#!/usr/bin/env python3
"""Remove idle duplicate Cargo slots using the repository's mkdir lock protocol."""

import argparse
from pathlib import Path
import re
import shutil


def clean_slots(root: Path, dry_run: bool) -> None:
    target = root / "target"
    locks = target / "rust-slot-locks"
    if target.is_symlink() or locks.is_symlink():
        raise RuntimeError("Refusing to clean slots through a symlink")
    if not target.is_dir():
        return
    if not dry_run:
        locks.mkdir(exist_ok=True)
    for slot in sorted(target.iterdir()):
        match = re.fullmatch(r"rust-slot-([1-9][0-9]*)", slot.name)
        if not match or slot.is_symlink() or not slot.is_dir():
            continue
        lock = locks / f"slot-{match[1]}.lock"
        if dry_run:
            state = "SKIP locked" if lock.exists() or lock.is_symlink() else "WOULD REMOVE"
            print(f"{state}: {slot}", flush=True)
            continue
        try:
            lock.mkdir()
        except FileExistsError:
            print(f"SKIP locked: {slot}", flush=True)
            continue
        try:
            print(f"REMOVE: {slot}", flush=True)
            shutil.rmtree(slot)
        finally:
            lock.rmdir()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    clean_slots(Path(__file__).resolve().parents[1], args.dry_run)
