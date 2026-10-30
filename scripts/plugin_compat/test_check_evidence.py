"""Black-box acceptance of the evidence CLI; expected results are hand-authored."""
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "crates/vw-agent/testdata/plugin_compat/t02"
SCRIPT = Path(__file__).with_name("check_evidence.py")


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.baseline = json.loads((FIXTURES / "baseline.json").read_text())
        self.evidence = json.loads((FIXTURES / "complete.json").read_text())

    def run_checker(self):
        with tempfile.TemporaryDirectory() as directory:
            baseline = Path(directory) / "baseline.json"
            evidence = Path(directory) / "evidence.json"
            baseline.write_text(json.dumps(self.baseline))
            evidence.write_text(json.dumps(self.evidence))
            output = subprocess.run(
                [sys.executable, str(SCRIPT), "--baseline", str(baseline),
                 "--evidence", str(evidence)], capture_output=True, text=True, check=False)
            self.assertEqual(json.loads(evidence.read_text()), self.evidence)
            return output

    def test_complete_evidence_passes(self):
        self.assertEqual(self.run_checker().returncode, 0)

    def test_missing_record_fails(self):
        del self.evidence["items"][0]["green"]
        result = self.run_checker()
        self.assertEqual(result.returncode, 1)
        self.assertIn("green", result.stderr)

    def test_missing_item_fails(self):
        self.evidence["items"] = []
        self.assertEqual(self.run_checker().returncode, 1)

    def test_duplicate_item_fails(self):
        self.evidence["items"].append(copy.deepcopy(self.evidence["items"][0]))
        self.assertEqual(self.run_checker().returncode, 1)

    def test_zero_tests_fails(self):
        self.evidence["items"][0]["green"]["effective_tests"] = 0
        self.assertEqual(self.run_checker().returncode, 1)

    def test_pending_regression_fails(self):
        self.evidence["items"][0]["conclusion"] = "已完成，待回归"
        self.assertEqual(self.run_checker().returncode, 1)

    def test_unverified_platform_fails(self):
        self.evidence["items"][0]["platforms"]["verified"] = ["macos"]
        self.assertEqual(self.run_checker().returncode, 1)

    def test_mock_public_service_fails(self):
        self.baseline["items"][0]["tasks"] = [62]
        item = self.evidence["items"][0]
        item["tasks"] = [62]
        item["external_prerequisites"] = [
            {"name": "public MCP", "status": "verified", "mode": "mock", "record": "stub"}]
        self.assertEqual(self.run_checker().returncode, 1)

    def test_source_hash_mismatch_fails(self):
        self.baseline["items"][0]["sources"][0]["sha256"] = "0" * 64
        result = self.run_checker()
        self.assertEqual(result.returncode, 1)
        self.assertIn("source hash mismatch", result.stderr)

    def test_missing_real_public_service_fails(self):
        self.baseline["items"][0]["tasks"] = [63]
        self.evidence["items"][0]["tasks"] = [63]
        result = self.run_checker()
        self.assertEqual(result.returncode, 1)
        self.assertIn("missing real public-service", result.stderr)

    def test_real_public_service_record_passes(self):
        self.baseline["items"][0]["tasks"] = [62]
        item = self.evidence["items"][0]
        item["tasks"] = [62]
        item["external_prerequisites"] = [
            {"name": "public MCP", "status": "verified", "mode": "real",
             "record": "Synthetic checker input; no real service claim"}]
        self.assertEqual(self.run_checker().returncode, 0)


if __name__ == "__main__":
    unittest.main()
