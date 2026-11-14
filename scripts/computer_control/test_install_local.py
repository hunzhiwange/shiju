"""Deferred installer behavior coverage; never changes the process HOME."""
from pathlib import Path
import tempfile
import unittest

from install_local import install, package, status, uninstall


class InstallTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="shiju install ")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.home = self.base / "user home"
        self.payload = self.base / "relocated payload"
        package(Path(__file__).resolve().parents[2], self.payload)

    def test_cc_control_21_relocated_payload_and_spaces_install(self):
        root = install(self.payload, self.home)
        self.assertTrue(status(self.home)["resources_present"])
        self.assertIn(str(root), (root / "chrome-native-host").read_text())
        self.assertTrue((root / "plugin/office/word/manifest.xml").is_file())

    def test_cc_control_21_repeated_install_is_idempotent(self):
        root = install(self.payload, self.home)
        before = (root / "installed-resources.json").read_bytes()
        install(self.payload, self.home)
        self.assertEqual(before, (root / "installed-resources.json").read_bytes())

    def test_cc_control_21_unknown_files_survive_uninstall(self):
        root = install(self.payload, self.home)
        unknown = root / "user-note"
        unknown.write_text("keep")
        uninstall(self.home)
        self.assertEqual(unknown.read_text(), "keep")
        self.assertFalse(status(self.home)["installed"])
        self.assertFalse((root / "chrome-native-host").exists())

    def test_cc_control_21_modified_owned_file_blocks_uninstall(self):
        root = install(self.payload, self.home)
        target = root / "chrome_native_host.py"
        target.write_text("user edit")
        with self.assertRaises(ValueError):
            uninstall(self.home)
        self.assertEqual(target.read_text(), "user edit")

    def test_cc_control_21_unknown_collision_is_preserved(self):
        root = self.home / ".shiju/computer-control"
        root.mkdir(parents=True, mode=0o700)
        target = root / "chrome_native_host.py"
        target.write_text("foreign")
        with self.assertRaises(ValueError):
            install(self.payload, self.home)
        self.assertEqual(target.read_text(), "foreign")

    def test_cc_control_21_symlink_parent_is_rejected(self):
        self.home.mkdir()
        outside = self.base / "outside"
        outside.mkdir()
        (self.home / ".shiju").symlink_to(outside, target_is_directory=True)
        with self.assertRaises(ValueError):
            install(self.payload, self.home)
        self.assertEqual(list(outside.iterdir()), [])

    def test_cc_control_21_tampered_package_is_rejected(self):
        (self.payload / "chrome_native_host.py").write_text("tampered")
        with self.assertRaises(ValueError):
            install(self.payload, self.home)
        self.assertFalse((self.home / ".shiju/computer-control").exists())

    def test_cc_control_21_missing_resource_is_reported(self):
        root = install(self.payload, self.home)
        (root / "chrome-native-host").unlink()
        self.assertFalse(status(self.home)["resources_present"])
