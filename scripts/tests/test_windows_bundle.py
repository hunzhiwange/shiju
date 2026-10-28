"""Check Windows package inputs without compiling or requiring WiX."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import zipfile


REPO = Path(__file__).resolve().parents[2]


class WindowsBundleTests(unittest.TestCase):
    def test_cross_build_requires_xwin_before_invoking_cargo(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            commands = root / "bin"
            commands.mkdir()
            for name, body in (
                ("uname", "echo Darwin"),
                ("cargo", 'echo "unexpected cargo invocation" >&2; exit 42'),
            ):
                command = commands / name
                command.write_text(f"#!/bin/sh\n{body}\n")
                command.chmod(0o755)
            env = dict(os.environ, PATH=str(commands), MODE="cli", CARGO_TARGET_DIR=str(root / "target"))
            result = subprocess.run(
                ["/bin/bash", str(REPO / "scripts/bundle_windows.sh")],
                cwd=root, env=env, capture_output=True, text=True, check=False,
            )
            self.assertEqual(result.returncode, 1)
            self.assertIn("cargo install cargo-xwin --locked", result.stderr)
            self.assertNotIn("unexpected cargo invocation", result.stderr)

    def test_desktop_package_rejects_missing_runtime_components(self):
        for option in ("INCLUDE_CLI_IN_APP", "INCLUDE_ACP_IN_APP"):
            with self.subTest(option=option), tempfile.TemporaryDirectory() as temp:
                env = dict(os.environ, MODE="app", OUT_DIR=temp)
                env[option] = "0"
                result = subprocess.run(
                    ["bash", str(REPO / "scripts/bundle_windows.sh")],
                    cwd=REPO, env=env, capture_output=True, text=True, check=False,
                )
                self.assertEqual(result.returncode, 1)
                self.assertIn("desktop packages require bundled shiju.exe and acp.exe", result.stderr)
                self.assertEqual(list(Path(temp).iterdir()), [])

    def test_packages_cargo_target_directory(self):
        for target in ("aarch64-pc-windows-msvc", "x86_64-pc-windows-msvc"):
            for override in (None, "relative", "absolute"):
                for mode in ("app", "cli"):
                    with self.subTest(target=target, override=override, mode=mode), tempfile.TemporaryDirectory() as temp:
                        root = Path(temp)
                        binaries = (
                            ("shi-ju.exe", "vw-webview.exe", "shiju.exe", "acp.exe")
                            if mode == "app" else ("shiju.exe", "vibe-agent.exe")
                        )
                        target_root = "target-dir" if override is None else "custom target"
                        binary_dir = root / target_root / target / "dist"
                        binary_dir.mkdir(parents=True)
                        for name in binaries:
                            (binary_dir / name).write_text("stale:" + name)
                        commands = root / "bin"
                        commands.mkdir()
                        cargo = commands / "cargo"
                        cargo.write_text("""#!/usr/bin/env python3
import os, sys
from pathlib import Path
args = sys.argv[1:]
root = Path(os.environ.get("CARGO_TARGET_DIR", "target-dir"))
target = args[args.index("--target") + 1]
profile = args[args.index("--profile") + 1] if "--profile" in args else "release"
output = root / target / profile
output.mkdir(parents=True, exist_ok=True)
for index, argument in enumerate(args):
    if argument == "--bin":
        name = args[index + 1] + ".exe"
        (output / name).write_text(name)
""")
                        cargo.chmod(0o755)
                        host = commands / "uname"
                        host.write_text("#!/bin/sh\necho Windows_NT\n")
                        host.chmod(0o755)
                        env = {key: value for key, value in os.environ.items() if key not in ("CARGO_TARGET_DIR", "INCLUDE_CLI_IN_APP", "INCLUDE_ACP_IN_APP")}
                        env.update(
                            PATH=f"{commands}:{env['PATH']}", TARGET=target, PROFILE="dist",
                            MODE=mode, OUT_DIR=str(root / "output"), CREATE_ZIP="1", CREATE_MSI="0",
                            INCLUDE_CLI_COMPAT_IN_APP="0",
                            WINDOWS_ICON_SOURCE=str(REPO / "assets/logo.png"),
                        )
                        if override:
                            env["CARGO_TARGET_DIR"] = (
                                str(root / target_root) if override == "absolute" else target_root
                            )
                        result = subprocess.run(
                            ["bash", str(REPO / "scripts/bundle_windows.sh")],
                            cwd=root, env=env, capture_output=True, text=True, check=False,
                        )
                        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                        archives = list((root / "output").glob("*.zip"))
                        self.assertEqual(len(archives), 1)
                        with zipfile.ZipFile(archives[0]) as archive:
                            for name in binaries:
                                self.assertEqual(archive.read(name).decode(), name)
                            self.assertIn("ShiJu.ico", archive.namelist())
                            if mode == "app":
                                launcher_bytes = archive.read("Start ShiJu Service.cmd")
                                self.assertNotIn(b"\r\r\n", launcher_bytes)
                                launcher = launcher_bytes.decode()
                                self.assertIn('if not exist "%~dp0shiju.exe"', launcher)
                                self.assertIn('"%~dp0shiju.exe" service install', launcher)
                                self.assertEqual(launcher.count('"%~dp0shiju.exe" service start'), 2)
                                self.assertIn("if errorlevel 1 goto failed", launcher)
                                self.assertNotIn("Start ShiJu Gateway.cmd", archive.namelist())


if __name__ == "__main__":
    unittest.main()
