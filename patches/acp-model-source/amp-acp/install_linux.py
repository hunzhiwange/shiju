"""Assemble the reviewed Amp ThreadWorker in a host-owned Linux stage."""
import base64
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tarfile


def read_package(archive, name, contract):
    required = {"package.json", contract["entry"]}
    with archive.open("rb") as source:
        digest = hashlib.sha512()
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
        integrity = "sha512-" + base64.b64encode(digest.digest()).decode("ascii")
        if integrity != contract["integrity"]:
            raise ValueError(f"Amp archive integrity mismatch: {name}")
        source.seek(0)
        with tarfile.open(fileobj=source, mode="r:gz") as package:
            files = {}
            for member in package:
                relative = member.name.removeprefix("package/")
                if member.name != f"package/{relative}" or relative not in required:
                    continue
                if not member.isfile() or relative in files:
                    raise ValueError(f"Invalid Amp archive member: {name}/{relative}")
                with package.extractfile(member) as content:
                    files[relative] = content.read()
    if files.keys() != required:
        raise ValueError(f"Missing Amp archive members: {name}")
    metadata = json.loads(files["package.json"])
    if (metadata.get("name"), metadata.get("version")) != (name, contract["version"]):
        raise ValueError(f"Amp package identity mismatch: {name}")
    return files


def install(runtime, adapter_archive, worker_archive):
    assets = Path(__file__).resolve().parent
    contract = json.loads((assets / "linux-runtime.json").read_text(encoding="utf-8"))
    target = f"{platform.machine()}-unknown-linux-gnu"
    if platform.system() != "Linux" or platform.libc_ver()[0] != "glibc" or target not in contract["targets"]:
        raise ValueError("Unsupported Amp Linux target")

    adapter_name, worker_name = "amp-acp", "@sourcegraph/amp"
    packages = {
        name: read_package(archive, name, contract["archives"][name])
        for name, archive in [(adapter_name, adapter_archive), (worker_name, worker_archive)]
    }
    runtime = runtime.parent.resolve(strict=True) / runtime.name
    runtime.mkdir(mode=0o700)
    modules = runtime / "node_modules"
    for name, files in packages.items():
        for relative, content in files.items():
            file = modules / name / relative
            file.parent.mkdir(parents=True, exist_ok=True)
            with file.open("xb") as output:
                output.write(content)

    adapter = modules / adapter_name
    subprocess.run(
        [sys.executable, assets / "apply.py", adapter, "--linux"],
        check=True,
        stdin=subprocess.DEVNULL,
    )
    for relative, expected in contract["files"].items():
        file = adapter / relative
        if file.resolve(strict=True) != Path(os.path.abspath(file)):
            raise ValueError(f"Redirected Amp runtime artifact: {relative}")
        if hashlib.sha256(file.read_bytes()).hexdigest() != expected:
            raise ValueError(f"Amp runtime artifact checksum mismatch: {relative}")

    entry = contract["archives"][adapter_name]["entry"]
    (adapter / entry).chmod(0o700)
    bin_dir = modules / ".bin"
    bin_dir.mkdir()
    (bin_dir / adapter_name).symlink_to(Path("..") / adapter_name / entry)


if __name__ == "__main__":
    install(*(Path(argument) for argument in sys.argv[1:]))
