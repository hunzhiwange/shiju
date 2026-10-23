"""Check Linux package inputs without compiling or requiring dpkg-deb."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


REPO = Path(__file__).resolve().parents[2]


class LinuxBundleTests(unittest.TestCase):
    def test_packages_cargo_target_directory(self):
        for target in ("aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu"):
            for override in (None, "relative", "absolute"):
                with self.subTest(target=target, override=override), tempfile.TemporaryDirectory() as temp:
                    root = Path(temp)
                    manifest = root / "crates/vw-desktop/Cargo.toml"
                    manifest.parent.mkdir(parents=True)
                    shutil.copyfile(REPO / "crates/vw-desktop/Cargo.toml", manifest)
                    binaries = ("shi-ju", "vw-webview", "shiju", "acp")
                    target_root = "target-dir" if override is None else "custom target"
                    binary_dir = root / target_root / target / "dist"
                    binary_dir.mkdir(parents=True)
                    for name in binaries:
                        (binary_dir / name).write_text(name)
                    icon = root / "icon.png"
                    icon.touch()
                    desktop = root / "shiju.desktop"
                    desktop.touch()
                    commands = root / "bin"
                    commands.mkdir()
                    dpkg = commands / "dpkg-deb"
                    dpkg.write_text(
                        '#!/bin/sh\nset -eu\n'
                        'cp -R "$3/usr/bin" "$CAPTURE_DIR"\n'
                        'cp "$3/DEBIAN/control" "$CAPTURE_DIR/control"\n'
                        'touch "$4"\n'
                    )
                    dpkg.chmod(0o755)
                    env = {key: value for key, value in os.environ.items() if key != "CARGO_TARGET_DIR"}
                    env.update(
                        PATH=f"{commands}:{env['PATH']}", TARGET=target, PROFILE="dist",
                        OUT_DIR=str(root / "output"), CREATE_DEB="1", CREATE_RPM="0",
                        PACKAGE_NAME="shiju", LINUX_ICON_SOURCE=str(icon),
                        LINUX_DESKTOP_SOURCE=str(desktop), CAPTURE_DIR=str(root / "captured"),
                    )
                    if override:
                        env["CARGO_TARGET_DIR"] = (
                            str(root / target_root) if override == "absolute" else target_root
                        )
                    result = subprocess.run(
                        ["bash", str(REPO / "scripts/bundle_linux.sh")],
                        cwd=root, env=env, capture_output=True, text=True, check=False,
                    )
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    for name in binaries:
                        self.assertEqual((root / "captured" / name).read_text(), name)
                    arch = "arm64" if target.startswith("aarch64") else "amd64"
                    self.assertIn(f"Architecture: {arch}\n", (root / "captured/control").read_text())
                    self.assertEqual(len(list((root / "output").glob("*.deb"))), 1)


if __name__ == "__main__":
    unittest.main()
