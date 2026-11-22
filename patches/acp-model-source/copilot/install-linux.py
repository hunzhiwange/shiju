"""Stage the fixed GNU Linux Copilot ACP payload in the managed npm prefix."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


def digest(path):
    checksum = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(chunk)
    return checksum.hexdigest()


def install(runtime, machine):
    assets = Path(__file__).resolve().parent
    contract = json.loads((assets / "linux-artifacts.json").read_text())
    target = contract["targets"][machine]
    version = contract["version"]
    modules = runtime / "node_modules"
    root = modules / contract["package"]

    lock = json.loads((runtime / "package-lock.json").read_text())["packages"]
    package = target["package"]
    for artifact in (contract, target):
        locked = lock["node_modules/" + artifact["package"]]
        if locked["version"] != version or locked["integrity"] != artifact["integrity"]:
            raise RuntimeError("Copilot Linux npm archive identity differs")

    native = modules / package / "copilot"
    if not native.is_file() or native.resolve(strict=True) != native:
        raise RuntimeError("Copilot Linux SEA path is redirected")
    if digest(native) != target["sea_sha256"]:
        raise RuntimeError("Copilot Linux SEA checksum differs")

    # The verified SEA has its own extraction mode and bundled Node runtime.
    # Its home and cache both belong to this disposable installer.
    cache = runtime / "sea-cache"
    cache.mkdir(mode=0o700)
    subprocess.run(
        [str(native), "--binary-version"],
        cwd=cache,
        env={"HOME": str(cache), "PATH": os.defpath, "COPILOT_SEA_EXTRACT_ONLY": "1",
             "COPILOT_PKG_CACHE_HOME": str(cache)},
        stdin=subprocess.DEVNULL,
        check=True,
        timeout=120,
    )
    extracted = cache / "pkg" / target["cache_platform"] / version
    embedded = root / "embedded"
    extracted.rename(embedded)
    subprocess.run(
        [sys.executable, "-I", str(assets / "apply.py"), str(embedded), "--linux"],
        stdin=subprocess.DEVNULL,
        check=True,
    )
    for name, expected in (contract["files"] | target["files"]).items():
        path = root / name
        if not path.is_file() or path.resolve(strict=True) != path or digest(path) != expected:
            raise RuntimeError("Copilot Linux payload identity differs: " + name)
    shutil.rmtree(cache)

    link = modules / ".bin/copilot"
    if not link.is_symlink() or link.readlink() != Path("../@github/copilot/npm-loader.js"):
        raise RuntimeError("Unexpected Copilot npm executable")
    link.unlink()
    link.symlink_to(Path("../@github/copilot") / contract["bin"])


if __name__ == "__main__":
    install(Path(sys.argv[1]).resolve(strict=True), sys.argv[2])
