"""Stage the reviewed GNU Vibe runtime inside the installer-owned directory."""

import hashlib
import json
import platform
from pathlib import Path
import shutil
import subprocess
import sys
import urllib.request


def verify(path, expected):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    if digest.hexdigest() != expected:
        raise RuntimeError(f"Vibe artifact checksum mismatch: {path.name}")


def run(*args):
    subprocess.run(args, check=True, stdin=subprocess.DEVNULL)


def main(assets, runtime, uv):
    target = f"{platform.machine()}-unknown-linux-gnu"
    manifest = json.loads((assets / "vibe/linux.json").read_text())
    if (platform.system() != "Linux" or platform.libc_ver()[0] != "glibc"
            or target not in manifest["targets"]):
        raise RuntimeError("Unsupported Vibe GNU runtime target")
    artifact = manifest["targets"][target]
    contract = json.loads((assets / "contracts.json").read_text())["vibe"]
    if manifest["version"] != contract["version"]:
        raise RuntimeError("Vibe Linux and shared Python contract versions differ")

    patch = assets / "vibe/mistral-vibe-3.0.4-session-binding.patch"
    launcher = assets / "vibe/launcher.sh"
    verify(patch, manifest["patch_sha256"])
    verify(launcher, manifest["launcher_sha256"])

    wheel_url = artifact["wheel_url"]
    wheel = assets / wheel_url.rsplit("/", 1)[1]
    with urllib.request.urlopen(wheel_url, timeout=60) as response, wheel.open("wb") as output:
        shutil.copyfileobj(response, output)
    verify(wheel, artifact["wheel_sha256"])

    run(uv, "venv", "--python", "3.12", "--relocatable", str(runtime))
    python = runtime / "bin/python"
    run(uv, "pip", "install", "--python", str(python), "--index-url",
        "https://pypi.org/simple", "--only-binary", ":all:", str(wheel))
    run(str(python), "-I", str(assets / "vibe/apply_patch.py"))
    site = runtime / "lib/python3.12/site-packages"
    for files in (contract["files"], artifact["files"]):
        for name, digest in files.items():
            verify(site / name, digest)

    # The generated console script has installer-dependent bytes. Replace it with
    # the fixed launcher, which calls the same upstream entry in isolated mode.
    entry = runtime / manifest["entry"]
    entry.unlink()
    shutil.copyfile(launcher, entry)
    entry.chmod(0o700)


if __name__ == "__main__":
    main(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3])
