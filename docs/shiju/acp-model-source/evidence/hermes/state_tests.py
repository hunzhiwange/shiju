#!/usr/bin/env python3
"""Exercise the real fixed package's history boundary in owned temporary roots."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

PYTHON = None
REFERENCE = "vw-session-model:" + "f" * 64
CODE = "from importlib.metadata import version; assert version('hermes-agent') == '0.19.0'; from acp_adapter.session_model_state import database_path; print(database_path(required=True))"


class HistoryBoundaryTests(unittest.TestCase):
    def execute(self, root, reference=REFERENCE):
        env = {key: os.environ[key] for key in ("PATH", "HOME") if key in os.environ}
        env.update(TMPDIR=str(root), VW_ACP_LAUNCH_REFERENCE=reference,
                   HERMES_HOME=str(root / "private-home"))
        return subprocess.run([str(PYTHON), "-c", CODE], env=env, capture_output=True, timeout=20)

    def test_private_state_survives_credential_home_removal(self):
        with tempfile.TemporaryDirectory(prefix="hermes-state-test-") as directory:
            root = Path(directory)
            home = root / "private-home"
            home.mkdir(mode=0o700)
            (home / "config.yaml").write_text("private runtime config")
            result = self.execute(root)
            self.assertEqual(result.returncode, 0)
            database = Path(result.stdout.decode().strip())
            self.assertEqual(database.stat().st_mode & 0o777, 0o600)
            self.assertEqual(database.parent.stat().st_mode & 0o777, 0o700)
            shutil.rmtree(home)
            self.assertTrue(database.exists())
            self.assertEqual(self.execute(root).stdout, result.stdout)

    def test_invalid_reference_rejects_without_creating_history(self):
        with tempfile.TemporaryDirectory(prefix="hermes-state-test-") as directory:
            root = Path(directory)
            self.assertNotEqual(self.execute(root, "../not-a-reference").returncode, 0)
            self.assertEqual(list(root.iterdir()), [])

    def test_history_symlink_is_rejected_without_following_target(self):
        with tempfile.TemporaryDirectory(prefix="hermes-state-test-") as directory:
            root = Path(directory)
            result = self.execute(root)
            self.assertEqual(result.returncode, 0)
            leaf = Path(result.stdout.decode().strip()).parent
            shutil.rmtree(leaf)
            outside = root / "outside"
            outside.mkdir(mode=0o700)
            leaf.symlink_to(outside, target_is_directory=True)
            self.assertNotEqual(self.execute(root).returncode, 0)
            self.assertFalse((outside / "state.db").exists())

    def test_world_readable_history_is_rejected(self):
        with tempfile.TemporaryDirectory(prefix="hermes-state-test-") as directory:
            root = Path(directory)
            result = self.execute(root)
            self.assertEqual(result.returncode, 0)
            Path(result.stdout.decode().strip()).chmod(0o644)
            self.assertNotEqual(self.execute(root).returncode, 0)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--python", required=True, type=Path)
    args, remaining = parser.parse_known_args()
    # Preserve the venv interpreter path: resolving its symlink loses sys.prefix.
    PYTHON = args.python.absolute()
    if not PYTHON.is_file():
        raise SystemExit("explicit Hermes Python does not exist")
    unittest.main(argv=[__file__, *remaining])
