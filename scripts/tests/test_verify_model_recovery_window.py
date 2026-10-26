"""Prevent a mixed Cargo build from launching the production desktop binary."""
import importlib.util
from pathlib import Path
import unittest

path = Path(__file__).resolve().parents[1] / "verify_model_recovery_window.py"
spec = importlib.util.spec_from_file_location("recovery_window", path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def artifact(name, kind, test):
    return {"reason": "compiler-artifact", "target": {"name": name, "kind": [kind]},
            "profile": {"test": test}, "executable": f"/slot/{name}"}


class ArtifactSelectionTests(unittest.TestCase):
    def test_regular_binary_after_test_is_never_selected(self):
        messages = [artifact("recovery-window", "test", True), artifact("shi-ju", "bin", False)]
        self.assertEqual(module.test_executable(messages, "recovery-window", "test"), Path("/slot/recovery-window"))

    def test_missing_or_non_test_artifact_fails_closed(self):
        for messages in [[], [artifact("recovery-window", "bin", False)], [artifact("recovery-window", "test", False)]]:
            with self.assertRaises(RuntimeError):
                module.test_executable(messages, "recovery-window", "test")


if __name__ == "__main__":
    unittest.main()
