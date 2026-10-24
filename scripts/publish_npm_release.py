#!/usr/bin/env python3
"""Publish verified existing npm installers; safely skip matching published versions."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import urllib.error
import urllib.parse
import urllib.request


def publish(directory, tag):
    root = Path(directory)
    manifest = json.loads((root / "dist-manifest.json").read_text())
    if manifest["announcement_tag"] != tag:
        raise SystemExit("Release manifest tag mismatch")
    expected_names = {"@shiju-ai/" + name for name in (
        "shiju", "shiju-acp", "shiju-desktop", "shiju-webview",
    )}
    packages = []
    for filename, item in manifest["artifacts"].items():
        if not filename.endswith("-npm-package.tar.gz"):
            continue
        path = root / filename
        data = path.read_bytes()
        if hashlib.sha256(data).hexdigest() != item["checksums"]["sha256"]:
            raise SystemExit(f"Release checksum mismatch: {filename}")
        with tarfile.open(path) as archive:
            package = json.load(archive.extractfile("package/package.json"))
        name, version = package["name"], package["version"]
        if name not in expected_names or version != tag.removeprefix("v"):
            raise SystemExit(f"Unexpected npm package: {name}@{version}")
        expected_names.remove(name)
        packages.append((path, name, version, hashlib.sha1(data).hexdigest()))
    if expected_names:
        raise SystemExit(f"Missing npm packages: {sorted(expected_names)}")
    # Validate every artifact before publishing any package.
    for path, name, version, digest in packages:
        url = "https://registry.npmjs.org/" + urllib.parse.quote(name, safe="") + "/" + version
        try:
            with urllib.request.urlopen(url, timeout=30) as response:
                published = json.load(response)
        except urllib.error.HTTPError as error:
            if error.code != 404:
                raise
            error.close()
        else:
            if published["dist"]["shasum"] != digest:
                raise SystemExit(f"Published package differs from release: {name}@{version}")
            print(f"Already published, checksum verified: {name}@{version}", flush=True)
            continue
        subprocess.run([
            "npm", "publish", str(path), "--access", "public", "--ignore-scripts",
            "--registry=https://registry.npmjs.org",
        ], check=True)


if __name__ == "__main__":
    publish(*sys.argv[1:])
