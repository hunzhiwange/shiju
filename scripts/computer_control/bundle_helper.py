#!/usr/bin/env python3
"""Stage ShiJu's native controller before inside-out macOS code signing."""
import argparse
from pathlib import Path
import plistlib

from install_local import checked, digest

APP_NAME = "ShiJu Computer Use.app"
BUNDLE_ID = "com.shiju.computer-use"
EXECUTABLE = "shiju-computer-use"
RELATIVE_APP = Path("Contents/Helpers") / APP_NAME
RECEIPT = ".shiju-bundle-inputs.plist"


def executable(app):
    """Resolve only the fixed, owned bundle layout; never fall back to PATH."""
    app = Path(app)
    info = plistlib.loads(checked(app, "Contents/Info.plist").read_bytes())
    if (info.get("CFBundleIdentifier") != BUNDLE_ID
            or info.get("CFBundleExecutable") != EXECUTABLE
            or info.get("LSUIElement") is not True):
        raise ValueError("computer control bundle identity mismatch")
    binary = checked(app, Path("Contents/MacOS") / EXECUTABLE)
    if not binary.is_file() or not binary.stat().st_mode & 0o111:
        raise ValueError("computer control executable missing or not executable")
    return binary


def stage(binary, icon, destination, version):
    binary, icon, destination = Path(binary), Path(icon), Path(destination)
    if not binary.is_file() or binary.is_symlink() or not binary.stat().st_mode & 0o111:
        raise ValueError("native control executable required")
    if not icon.is_file() or icon.is_symlink():
        raise ValueError("ShiJu product icon required")
    if not version.strip():
        raise ValueError("bundle version required")
    info = {
        "CFBundleIdentifier": BUNDLE_ID, "CFBundleExecutable": EXECUTABLE,
        "CFBundleName": "ShiJu Computer Use", "CFBundleDisplayName": "ShiJu Computer Use",
        "CFBundlePackageType": "APPL", "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleShortVersionString": version, "CFBundleVersion": version,
        "CFBundleIconFile": "ShiJu.icns", "LSUIElement": True,
        "NSHighResolutionCapable": True,
    }
    files = {"Contents/Info.plist": plistlib.dumps(info),
             f"Contents/MacOS/{EXECUTABLE}": binary.read_bytes(),
             "Contents/Resources/ShiJu.icns": icon.read_bytes()}
    # Stage only into a new or exactly owned input directory. Signed bundles are
    # immutable here: signing changes the executable and must use a fresh stage.
    receipt = checked(destination, RECEIPT)
    owned = plistlib.loads(receipt.read_bytes()) if receipt.exists() else {}
    for name, expected in owned.items():
        target = checked(destination, name)
        if target.exists() and (not target.is_file() or digest(target.read_bytes()) != expected):
            raise ValueError("bundle input changed; preserve and use a fresh stage")
    if destination.exists():
        for path in destination.rglob("*"):
            if path.is_symlink():
                raise ValueError("symlink in control bundle")
            if path.is_file() and path != receipt and path.relative_to(destination).as_posix() not in owned:
                raise ValueError("unknown control bundle file; preserve and use a fresh stage")
    for name, data in files.items():
        target = checked(destination, name)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
        target.chmod(0o755 if name.endswith('/' + EXECUTABLE) else 0o644)
    for name in owned.keys() - files.keys():
        checked(destination, name).unlink(missing_ok=True)
    receipt.write_bytes(plistlib.dumps({name: digest(data) for name, data in files.items()}))
    return executable(destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--icon', type=Path, required=True)
    parser.add_argument('--destination', type=Path, required=True)
    parser.add_argument('--version', required=True)
    args = parser.parse_args()
    stage(args.binary, args.icon, args.destination, args.version)


if __name__ == '__main__':
    main()
