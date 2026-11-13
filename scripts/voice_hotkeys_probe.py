#!/usr/bin/env python3
"""Explicit macOS hotkey probe against production sources; no microphone or service.

Opens only its own temporary input window. Existing Input Monitoring and
Accessibility authorization are required; this script never requests either.
All Rust processes and artifacts use the repository Cargo slot wrapper.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "scripts/voice_hotkeys_probe"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", choices=("shared", "null"), default="shared")
    parser.add_argument("--suppression", choices=("default", "none"), default="none")
    parser.add_argument("--double-taps", type=int, default=10)
    args = parser.parse_args()
    if not 1 <= args.double_taps <= 100:
        parser.error("--double-taps must be between 1 and 100")
    if sys.platform != "darwin":
        parser.error("macOS is required")
    if os.environ.get("SHIJU_CARGO_SLOT_ACTIVE") != "1":
        return subprocess.call([
            sys.executable, str(ROOT / "scripts/cargo_slot.py"),
            sys.executable, str(Path(__file__).resolve()), *sys.argv[1:],
        ], cwd=ROOT)

    with tempfile.TemporaryDirectory(prefix="shiju-hotkeys-probe-") as temporary:
        work = Path(temporary)
        (work / "src").mkdir()
        modules = []
        for name in ("hotkeys", "hotkeys_macos", "permissions", "permissions_macos",
                     "preferences", "local"):
            source = ROOT / "crates/vw-desktop/src/app/voice" / f"{name}.rs"
            # Imported desktop modules normally inherit the desktop crate's Clippy policy.
            modules.append(f"#[allow(clippy::precedence)] #[path={json.dumps(str(source), ensure_ascii=False)}] pub mod {name};")
        source = (FIXTURES / "main.rs").read_text().replace(
            "// @VOICE_MODULES@", "\n".join(modules))
        (work / "src/main.rs").write_text(source)
        # Reuse locked workspace dependency versions; offline resolution never downloads.
        shutil.copyfile(ROOT / "Cargo.lock", work / "Cargo.lock")
        manifest = '''[package]
name = "shiju-hotkeys-probe"
version = "0.0.0"
edition = "2024"
[profile.dev]
opt-level = 1
debug = "line-tables-only"
[dependencies]
anyhow = "1"
block2 = "0.6"
objc2 = "0.6"
objc2-foundation = { version = "0.3", default-features = false, features = ["std", "NSString"] }
open = "5"
directories = "5"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
iced = { version = "0.14", features = ["image", "svg", "advanced", "tokio", "highlighter", "markdown"] }
'''
        manifest += f'vw-config-types = {{ path = {json.dumps(str(ROOT / "crates/vw-config-types"), ensure_ascii=False)} }}\n'
        manifest += '[patch.crates-io]\n'
        for name in ("block", "iced_winit"):
            manifest += f'{name} = {{ path = {json.dumps(str(ROOT / "vendor" / name), ensure_ascii=False)} }}\n'
        (work / "Cargo.toml").write_text(manifest)
        target = work / "input-target"
        subprocess.run(["swiftc", str(FIXTURES / "target.swift"), "-o", str(target)], check=True)
        env = os.environ.copy()
        env["SHIJU_HOTKEY_TARGET"] = str(target)
        env["SHIJU_HOTKEY_DOUBLE_TAPS"] = str(args.double_taps)
        for name in ("SHIJU_HOTKEY_NULL_SOURCE", "SHIJU_HOTKEY_DEFAULT_SUPPRESSION"):
            env.pop(name, None)
        if args.source == "null":
            env["SHIJU_HOTKEY_NULL_SOURCE"] = "1"
        if args.suppression == "default":
            env["SHIJU_HOTKEY_DEFAULT_SUPPRESSION"] = "1"
        print(f"source={args.source} suppression={args.suppression}", flush=True)
        subprocess.run([
            "cargo", "clippy", "--offline", "--manifest-path", str(work / "Cargo.toml"),
            "--", "-D", "warnings",
        ], cwd=ROOT, env=env, check=True)
        return subprocess.call([
            "cargo", "run", "--offline", "--manifest-path", str(work / "Cargo.toml"),
        ], cwd=ROOT, env=env)


if __name__ == "__main__":
    sys.exit(main())
