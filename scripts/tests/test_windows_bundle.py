"""Check Windows package inputs without compiling or requiring WiX."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import zipfile


REPO = Path(__file__).resolve().parents[2]


class WindowsBundleTests(unittest.TestCase):
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
                            (binary_dir / name).write_text(name)
                        commands = root / "bin"
                        commands.mkdir()
                        cargo = commands / "cargo"
                        cargo.write_text('#!/bin/sh\necho "unexpected rebuild" >&2\nexit 1\n')
                        cargo.chmod(0o755)
                        env = {key: value for key, value in os.environ.items() if key != "CARGO_TARGET_DIR"}
                        env.update(
                            PATH=f"{commands}:{env['PATH']}", TARGET=target, PROFILE="dist",
                            MODE=mode, OUT_DIR=str(root / "output"), CREATE_ZIP="1", CREATE_MSI="0",
                            INCLUDE_CLI_IN_APP="1", INCLUDE_ACP_IN_APP="1", INCLUDE_CLI_COMPAT_IN_APP="0",
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
                                self.assertIn("Start ShiJu Gateway.cmd", archive.namelist())


if __name__ == "__main__":
    unittest.main()
