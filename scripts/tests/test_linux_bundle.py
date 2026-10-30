"""Check Linux package inputs without compiling or requiring dpkg-deb."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


REPO = Path(__file__).resolve().parents[2]


class LinuxBundleTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which("dpkg-deb"), "requires native Debian package tools")
    def test_real_deb_contains_complete_installation_for_both_architectures(self):
        for target, arch in [("aarch64-unknown-linux-gnu", "arm64"), ("x86_64-unknown-linux-gnu", "amd64")]:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                manifest = root / "crates/vw-desktop/Cargo.toml"
                manifest.parent.mkdir(parents=True)
                shutil.copyfile(REPO / "crates/vw-desktop/Cargo.toml", manifest)
                binaries = ("shi-ju", "vw-webview", "shiju", "acp")
                binary_dir = root / "target-dir" / target / "dist"
                binary_dir.mkdir(parents=True)
                for name in binaries:
                    (binary_dir / name).write_bytes(b"neutral fixture " + name.encode())
                env = dict(os.environ, TARGET=target, PROFILE="dist", CREATE_DEB="1", CREATE_RPM="0",
                           PACKAGE_NAME="shiju", CARGO_TARGET_DIR=str(root / "target-dir"),
                           OUT_DIR=str(root / "output"), LINUX_ICON_SOURCE=str(REPO / "assets/logo.png"),
                           LINUX_DESKTOP_SOURCE=str(REPO / "release/linux/vibewindow.desktop"))
                subprocess.run(["bash", str(REPO / "scripts/bundle_linux.sh")], cwd=root, env=env, check=True, capture_output=True)
                packages = list((root / "output").glob("*.deb"))
                self.assertEqual(len(packages), 1)
                package = packages[0]
                metadata = subprocess.check_output(["dpkg-deb", "-W", "--showformat=${Package}\n${Architecture}", str(package)], text=True)
                self.assertEqual(metadata, f"shiju\n{arch}")
                extracted = root / "extracted"
                subprocess.run(["dpkg-deb", "-R", str(package), str(extracted)], check=True)
                for name in binaries:
                    self.assertEqual((extracted / "usr/bin" / name).read_bytes(), (binary_dir / name).read_bytes())
                    self.assertTrue((extracted / "usr/bin" / name).stat().st_mode & 0o111)
                self.assertTrue((extracted / "usr/share/applications/shiju.desktop").is_file())
                self.assertTrue((extracted / "usr/share/icons/hicolor/256x256/apps/shiju.png").is_file())
                self.assertTrue((extracted / "DEBIAN/postinst").stat().st_mode & 0o111)

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
