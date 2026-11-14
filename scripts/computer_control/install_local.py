#!/usr/bin/env python3
"""Package resources and manage only hash-owned ShiJu user files (macOS)."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import sys

HOST = "com.shiju.computer_control"
ORIGIN = "chrome-extension://ogfgcnpdhnmkpcbaclbglefaclanipmg/"
RECEIPT = "installed-resources.json"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def checked(root, relative):
    path = Path(relative)
    if path.is_absolute() or not path.parts or any(p in (".", "..") for p in path.parts):
        raise ValueError("invalid resource path")
    target = root / path
    for parent in (target, *target.parents):
        if parent.is_symlink():
            raise ValueError("symlink resource path")
    return target


def package(source, destination):
    """Stage portable resources before app signing; no developer paths in payload."""
    source, destination = Path(source), Path(destination)
    files = {}
    plugin = source / "plugins/shiju-computer-control"
    for path in sorted(plugin.rglob("*")):
        if path.is_symlink():
            raise ValueError("symlink package resource")
        if path.is_file() and not path.name.endswith((".test.mjs",)):
            files["plugin/" + path.relative_to(plugin).as_posix()] = path.read_bytes()
    for name in ("chrome_native_host.py", "install_local.py"):
        files[name] = (source / "scripts/computer_control" / name).read_bytes()
    for relative, data in files.items():
        target = checked(destination, relative)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    (destination / "resources.json").write_text(json.dumps(
        {name: digest(data) for name, data in files.items()}, sort_keys=True, indent=2) + "\n")


def root_for(home):
    home = Path(home).absolute()
    root = checked(home, ".shiju/computer-control")
    if root.exists() and (root.stat().st_uid != os.geteuid() or root.stat().st_mode & 0o077):
        raise ValueError("unsafe ShiJu directory")
    return root


def inventory(root):
    receipt = checked(root, RECEIPT)
    if not receipt.exists():
        return {}
    records = json.loads(receipt.read_text())
    if not isinstance(records, dict):
        raise ValueError("invalid ownership receipt")
    for name, expected in records.items():
        target = checked(root, name)
        if not isinstance(expected, str) or (target.exists() and digest(target.read_bytes()) != expected):
            raise ValueError("owned resource changed; preserve and resolve manually")
    return records


def install(payload, home):
    payload = Path(payload).absolute()
    files = {}
    for name, expected in json.loads((payload / "resources.json").read_text()).items():
        data = checked(payload, name).read_bytes()
        if digest(data) != expected:
            raise ValueError("package hash mismatch")
        files[name] = data
    root = root_for(home)
    # Use a shell launcher so Python paths containing spaces remain valid.
    files["chrome-native-host"] = (
        "#!/bin/sh\nexec " + shlex.quote(sys.executable) + " " +
        shlex.quote(str(root / "chrome_native_host.py")) + ' "$@"\n').encode()
    files[f"chrome-profile/NativeMessagingHosts/{HOST}.json"] = json.dumps({
        "name": HOST, "description": "ShiJu Chrome frame relay", "type": "stdio",
        "path": str(root / "chrome-native-host"), "allowed_origins": [ORIGIN],
    }, indent=2).encode()
    owned = inventory(root)
    # Preflight all collisions before mutating anything; unknown files stay untouched.
    for name in files:
        target = checked(root, name)
        if target.exists() and name not in owned:
            raise ValueError("resource is not owned by this installer")
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    for name, data in files.items():
        target = checked(root, name)
        target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        target.write_bytes(data)
        target.chmod(0o700 if name == "chrome-native-host" else 0o600)
    # Remove obsolete files only when the previous receipt still matches them.
    for name in owned.keys() - files.keys():
        checked(root, name).unlink(missing_ok=True)
    receipt = checked(root, RECEIPT)
    receipt.write_text(json.dumps({name: digest(data) for name, data in files.items()}, indent=2) + "\n")
    receipt.chmod(0o600)
    return root


def uninstall(home):
    root = root_for(home)
    owned = inventory(root)
    for name in owned:
        checked(root, name).unlink(missing_ok=True)
    checked(root, RECEIPT).unlink(missing_ok=True)
    # Deliberately retain directories, profiles, sockets and unknown user files.


def status(home):
    root = root_for(home)
    owned = inventory(root)
    return {"installed": bool(owned), "resources_present": bool(owned) and all(
        checked(root, name).is_file() for name in owned)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("stage", "install", "uninstall", "status"))
    parser.add_argument("--home", type=Path, default=Path.home())
    parser.add_argument("--payload", type=Path, default=Path(__file__).resolve().parent)
    parser.add_argument("--source", type=Path)
    parser.add_argument("--destination", type=Path)
    args = parser.parse_args()
    if args.action == "stage":
        if args.source is None or args.destination is None:
            parser.error("stage requires --source and --destination")
        package(args.source, args.destination)
    elif args.action == "install":
        if sys.platform != "darwin":
            parser.error("local installation currently supports macOS only")
        print(install(args.payload, args.home))
    elif args.action == "uninstall":
        uninstall(args.home)
    else:
        print(json.dumps(status(args.home)))


if __name__ == "__main__":
    main()
