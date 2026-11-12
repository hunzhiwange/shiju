import importlib.util
from pathlib import Path
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location(
    "clean_cargo_slots", Path(__file__).resolve().parents[1] / "clean_cargo_slots.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class CleanCargoSlotsTests(unittest.TestCase):
    def test_cleanup_preserves_locked_slots_primary_cache_and_symlink_target(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            target = root / "target"
            locks = target / "rust-slot-locks"
            locks.mkdir(parents=True)
            (locks / "slot-2.lock").mkdir()
            for name in ("rust-slot-1", "rust-slot-2", "debug", "evidence"):
                (target / name).mkdir()
                (target / name / "keep").write_text("data")
            outside = root / "outside"
            outside.mkdir()
            (target / "rust-slot-3").symlink_to(outside, target_is_directory=True)
            MODULE.clean_slots(root, dry_run=False)
            self.assertFalse((target / "rust-slot-1").exists())
            self.assertFalse((locks / "slot-1.lock").exists())
            for name in ("rust-slot-2", "debug", "evidence"):
                self.assertTrue((target / name / "keep").is_file())
            self.assertTrue(outside.is_dir())
            self.assertTrue((locks / "slot-2.lock").is_dir())

    def test_preview_does_not_create_locks_or_delete_slots(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            slot = root / "target" / "rust-slot-1"
            slot.mkdir(parents=True)
            MODULE.clean_slots(root, dry_run=True)
            self.assertTrue(slot.is_dir())
            self.assertFalse((root / "target" / "rust-slot-locks").exists())

    def test_cleanup_rejects_symlinked_target(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "outside").mkdir()
            (root / "target").symlink_to(root / "outside", target_is_directory=True)
            with self.assertRaises(RuntimeError):
                MODULE.clean_slots(root, dry_run=False)


if __name__ == "__main__":
    unittest.main()
