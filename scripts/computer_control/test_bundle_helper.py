"""Deferred controller packaging coverage; no OS signing or TCC substitutes."""
from pathlib import Path
import plistlib
import tempfile
import unittest

from bundle_helper import BUNDLE_ID, EXECUTABLE, RELATIVE_APP, executable, stage
from install_local import helper_path, install, package, uninstall


class BundleHelperTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="shiju helper ")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.product = self.root / "ShiJu.app"
        self.app = self.product / RELATIVE_APP
        self.binary = self.root / EXECUTABLE
        self.binary.write_bytes(b"native-input")
        self.binary.chmod(0o755)
        self.icon = self.root / "ShiJu.icns"
        self.icon.write_bytes(b"product-icon")

    def stage(self, version="1.0"):
        return stage(self.binary, self.icon, self.app, version)

    def test_fixed_identity_agent_and_product_icon(self):
        binary = self.stage()
        info = plistlib.loads((self.app / "Contents/Info.plist").read_bytes())
        self.assertEqual(info["CFBundleIdentifier"], BUNDLE_ID)
        self.assertIs(info["LSUIElement"], True)
        self.assertEqual(binary.read_bytes(), self.binary.read_bytes())
        self.assertEqual((self.app / "Contents/Resources/ShiJu.icns").read_bytes(), self.icon.read_bytes())
        self.stage("2.0")
        self.assertEqual(executable(self.app), binary)
        self.assertEqual(plistlib.loads((self.app / "Contents/Info.plist").read_bytes())["CFBundleIdentifier"], BUNDLE_ID)

    def test_missing_bundle_fails_without_path_fallback(self):
        with self.assertRaises(FileNotFoundError):
            executable(self.app)

    def test_missing_or_nonexecutable_input_fails(self):
        self.binary.chmod(0o644)
        with self.assertRaises(ValueError):
            self.stage()
        self.assertFalse(self.app.exists())

    def test_identity_drift_rejected(self):
        self.stage()
        path = self.app / "Contents/Info.plist"
        info = plistlib.loads(path.read_bytes())
        info["CFBundleIdentifier"] = "unrelated.product"
        path.write_bytes(plistlib.dumps(info))
        with self.assertRaises(ValueError):
            executable(self.app)
        with self.assertRaises(ValueError):
            self.stage()

    def test_unknown_file_preserved_and_update_rejected(self):
        self.stage()
        unknown = self.app / "user-file"
        unknown.write_text("keep")
        with self.assertRaises(ValueError):
            self.stage("2.0")
        self.assertEqual(unknown.read_text(), "keep")

    def test_symlink_destination_rejected(self):
        self.app.parent.mkdir(parents=True)
        self.app.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(ValueError):
            self.stage()

    def test_discover_relocated_bundle(self):
        binary = self.stage()
        payload = self.product / "Contents/Resources/computer-control"
        self.assertEqual(helper_path(payload), binary)
        with self.assertRaises(ValueError):
            helper_path(self.root)

    def test_local_uninstall_preserves_shipped_app_and_unknown_files(self):
        binary = self.stage()
        payload = self.product / "Contents/Resources/computer-control"
        package(Path(__file__).resolve().parents[2], payload)
        home = self.root / "home"
        root = install(payload, home)
        unknown = root / "user-file"
        unknown.write_text("keep")
        uninstall(home)
        self.assertEqual(unknown.read_text(), "keep")
        self.assertEqual(binary.read_bytes(), self.binary.read_bytes())
