"""Copilot fixed identity boundaries; mocks here are not CLI acceptance evidence."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import verify_acp_model_source as runner

ROOT = Path(__file__).resolve().parents[1]


class CopilotIdentity(unittest.TestCase):
    def entry(self):
        return copy.deepcopy(next(e for e in json.loads(
            (ROOT / "docs/shiju/acp-model-source/cli-versions.json").read_text())["entries"]
            if e["key"] == "copilot"))

    def test_fixed_version_period_is_accepted_and_native_artifact_is_returned(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "node_modules/@github/copilot"
            native = package / "node_modules/@github/copilot-darwin-arm64"
            native.mkdir(parents=True)
            (package / "package.json").write_text(json.dumps({"name": "@github/copilot", "version": "1.0.105"}))
            (native / "package.json").write_text(json.dumps({"name": "@github/copilot-darwin-arm64", "version": "1.0.105"}))
            loader = package / "npm-loader.js"
            loader.touch()
            binary = native / "copilot"
            binary.touch()
            def probe(argv, **kwargs):
                if argv[0] == "/node":
                    return subprocess.CompletedProcess(argv, 0, (str(binary) + "\n").encode(), b"")
                return subprocess.CompletedProcess(argv, 0, b"GitHub Copilot CLI 1.0.105.\nRun 'copilot update' to check for updates.\n", b"")
            with patch.object(runner.shutil, "which", side_effect=lambda name, **kw: "/node" if name == "node" else str(loader)), \
                 patch.object(runner.subprocess, "run", side_effect=probe), \
                 patch.object(runner.platform, "system", return_value="Darwin"), \
                 patch.object(runner.platform, "machine", return_value="arm64"):
                result = runner.locate(self.entry(), {"PATH": ""})
            self.assertEqual(result["command"], str(binary.resolve()))
            self.assertEqual(result["args"], ["--acp", "--stdio"])

    def test_version_prefix_extended_release_and_wrong_product_are_rejected(self):
        for line in ("GitHub Copilot CLI 1.0.105.1.", "GitHub Copilot CLI 1.0.105-beta.",
                     "GitHub Copilot CLI 11.0.105.", "Other CLI 1.0.105.",
                     "GitHub Copilot CLI 1.0.106."):
            with self.subTest(line=line), \
                 patch.object(runner.shutil, "which", return_value="/copilot"), \
                 patch.object(runner.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, line.encode(), b"")):
                with self.assertRaises(runner.VersionMismatch):
                    runner.locate(self.entry(), {"PATH": ""})

    def test_native_package_mismatch_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            binary = Path(temporary) / "copilot"
            binary.touch()
            metadata = binary.parent / "package.json"
            for name, version in (("@github/copilot-darwin-arm64", "1.0.106"),
                                  ("unrelated-package", "1.0.105")):
                metadata.write_text(json.dumps({"name": name, "version": version}))
                with self.subTest(name=name, version=version), \
                     patch.object(runner.platform, "system", return_value="Darwin"), \
                     patch.object(runner.platform, "machine", return_value="arm64"):
                    with self.assertRaises(runner.VersionMismatch):
                        runner.copilot_native_executable(binary, "1.0.105", {})

    def test_managed_native_addon_tampering_is_rejected(self):
        import hashlib
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            embedded = package / "embedded"
            embedded.mkdir(parents=True)
            for parent in (package, embedded):
                (parent / "package.json").write_text(json.dumps({"name": "@github/copilot", "version": "1.0.105"}))
            entry = embedded / "index.js"
            addon = embedded / "runtime.node"
            entry.write_bytes(b"synthetic entry; never executed")
            addon.write_bytes(b"synthetic addon; never loaded")
            contracts = root / "crates/vw-acp/src/managed_runtime/contracts.json"
            contracts.parent.mkdir(parents=True)
            contracts.write_text(json.dumps({"copilot": {"package": "@github/copilot", "version": "1.0.105",
                "files": {str(p.relative_to(package)): hashlib.sha256(p.read_bytes()).hexdigest() for p in (entry, addon)}}}))
            with patch.object(runner, "ROOT", root), \
                 patch.object(runner.platform, "system", return_value="Darwin"), \
                 patch.object(runner.platform, "machine", return_value="arm64"):
                self.assertEqual(runner.copilot_native_executable(entry, "1.0.105", {}), str(entry.absolute()))
                addon.write_bytes(b"tampered addon")
                with self.assertRaisesRegex(runner.VersionMismatch, "checksum differs"):
                    runner.copilot_native_executable(entry, "1.0.105", {})

    def test_other_platforms_do_not_inherit_native_identity_claim(self):
        with patch.object(runner.platform, "system", return_value="Linux"), \
             patch.object(runner.platform, "machine", return_value="x86_64"):
            with self.assertRaisesRegex(runner.Unavailable, "unverified on this platform"):
                runner.copilot_native_executable("/never/executed", "1.0.105", {})


if __name__ == "__main__":
    unittest.main()
