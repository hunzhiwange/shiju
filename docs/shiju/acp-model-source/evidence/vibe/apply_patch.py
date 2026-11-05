#!/usr/bin/env python3
"""Apply the reviewed 3.0.4 patch inside an explicitly selected virtualenv."""
import hashlib
import importlib.metadata
import json
from pathlib import Path
import subprocess
import sys
import sysconfig

ROOT = Path(__file__).resolve().parent

def main():
    if sys.prefix == sys.base_prefix or importlib.metadata.version("mistral-vibe") != "3.0.4":
        raise SystemExit("requires an isolated mistral-vibe==3.0.4 virtualenv")
    site = Path(sysconfig.get_paths()["purelib"])
    hashes = json.loads((ROOT / "upstream-hashes.json").read_text())
    for name, expected in hashes.items():
        path = site / name
        if (not path.exists() and expected is not None) or (path.exists() and (
                expected is None or hashlib.sha256(path.read_bytes()).hexdigest() != expected)):
            raise SystemExit("upstream file differs from reviewed 3.0.4: " + name)
    patch = ROOT / "mistral-vibe-3.0.4-session-binding.patch"
    command = ["patch", "--batch", "--forward", "-p1", "-d", str(site), "-i", str(patch)]
    subprocess.run(command + ["--dry-run"], check=True)
    subprocess.run(command, check=True)
    print("Applied session binding and isolated skill scanner to mistral-vibe 3.0.4")

if __name__ == "__main__":
    main()
