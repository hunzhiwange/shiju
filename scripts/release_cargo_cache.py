"""Emit the Cargo cache key and paths shared by release and cache warming."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import tomllib


def main():
    target_dir = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
    metadata = json.loads(
        subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version=1"])
    )
    cargo_dirs = ("target_directory", "build_directory")
    if any(Path(metadata[key]).resolve() != target_dir for key in cargo_dirs):
        raise SystemExit("Cargo target-dir and build-dir must match the held slot")

    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).resolve()
    targets = os.environ["CACHE_TARGETS"].split()
    rust = subprocess.check_output(["rustc", "-Vv"])
    config = {
        "cargo": tomllib.loads(Path(".cargo/config.toml").read_text()),
        "toolchain": tomllib.loads(Path("rust-toolchain.toml").read_text()),
        "dist": tomllib.loads(Path("dist-workspace.toml").read_text()),
        "profile": tomllib.loads(Path("Cargo.toml").read_text()).get("profile", {}),
        "flags": [os.environ.get(key, "") for key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS")],
    }
    config["cargo"].pop("alias", None)
    fingerprint = hashlib.sha256(rust + json.dumps(config, sort_keys=True).encode()).hexdigest()[:24]
    platform = "-".join([os.environ["RUNNER_OS"], os.environ["RUNNER_ARCH"], *targets])
    prefix = f"release-rust-v3-{platform}-{fingerprint}-"

    lockfile = tomllib.loads(Path("Cargo.lock").read_text())
    # Workspace version bumps do not change the set of registry and Git dependencies.
    external_packages = [package for package in lockfile["package"] if "source" in package]
    locked_sources = json.dumps(external_packages, sort_keys=True).encode()
    dependencies = hashlib.sha256(locked_sources).hexdigest()[:16]
    cache_paths = [
        cargo_home / path
        for path in ("registry/index", "registry/cache", "registry/src", "git/db", "git/checkouts")
    ]
    profiles = [target_dir / "dist", *(target_dir / target / "dist" for target in targets)]
    cache_paths.extend(
        profile / directory
        for profile in profiles
        for directory in ("deps", "build", ".fingerprint", "incremental")
    )

    with Path(os.environ["GITHUB_OUTPUT"]).open("a") as output:
        print(f"key={prefix}{dependencies}", f"prefix={prefix}", "paths<<EOF", sep="\n", file=output)
        for path in cache_paths:
            print(path, file=output)
        print("EOF", file=output)
    print(rust.decode(), f"Cargo cache slot: {target_dir}")


if __name__ == "__main__":
    main()
