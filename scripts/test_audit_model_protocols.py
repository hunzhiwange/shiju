"""Behavioral fixtures for the catalog inventory and research contract."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest

from audit_model_protocols import build_groups, build_inventory, check_research, load_json


class CatalogAuditTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        model = {"id": "deployment", "canonical_model_id": "maker/same",
                 "modalities": {"input": ["text"], "output": ["text"]}}
        self.api = root / "api.json"
        self.shiju = root / "shiju.json"
        self.api.write_text(json.dumps({
            "a": {"npm": "@ai-sdk/openai-compatible", "api": "https://a/v1",
                  "models": {"key": {**model, "provider": {
                      "api": "wss://a/realtime", "npm": "@ai-sdk/google",
                      "shape": "realtime"}}}},
            "b": {"adapter": "anthropic", "models": {"key": model}},
            "shiju": {"models": {"old": model}}}))
        self.shiju.write_text(json.dumps({
            "shiju": {"adapter": "openai-compatible", "api": "https://s/v1",
                      "models": {"audio": {"id": "tts", "modalities": {
                          "input": ["text"], "output": ["audio"]}}}},
            "ignored": {"models": {"key": model}}}))
        self.inv = build_inventory([self.api, self.shiju])
        self.seed = {"research_groups": [
            {"group": 1, "task": 2, "providers": ["a", "shiju"]},
            {"group": 2, "task": 3, "providers": ["b", "removed"]}]}

    def test_source_identity_and_canonical_duplicates_are_preserved(self):
        rows = self.inv["records"]
        self.assertEqual(len(rows), 5)
        self.assertEqual(len({tuple(r[k] for k in ("source", "provider_id", "model_key"))
                              for r in rows}), 5)
        self.assertEqual(sum(r["canonical_model_id"] == "maker/same" for r in rows), 4)
        self.assertEqual(rows[0]["model_api_id"], "deployment")

    def test_model_protocol_fields_override_provider_individually(self):
        row = self.inv["records"][0]
        self.assertEqual(row["effective_declarations"], {
            "api": "wss://a/realtime", "npm": "@ai-sdk/google", "shape": "realtime"})
        self.assertEqual(row["provider_declarations"]["npm"], "@ai-sdk/openai-compatible")
        self.assertIn("model_provider_fields_not_loaded", row["metadata_gaps"])

    def test_missing_endpoint_is_not_guessed_from_sdk(self):
        row = next(r for r in self.inv["records"] if r["provider_id"] == "b")
        self.assertNotIn("api", row["effective_declarations"])
        self.assertIn("default_endpoint_unverified", row["metadata_gaps"])
        self.assertEqual(row["effective_declarations"]["adapter"], "anthropic")

    def test_non_text_and_realtime_need_separate_review(self):
        rows = self.inv["records"]
        self.assertEqual(next(r for r in rows if r["model_key"] == "audio")
                         ["text_acp_applicability"], "no_text_output")
        self.assertEqual(rows[0]["text_acp_applicability"], "specialized_transport_review")

    def test_overlay_replaces_only_shiju_without_dropping_source_rows(self):
        old = next(r for r in self.inv["records"] if r["model_key"] == "old")
        ignored = next(r for r in self.inv["records"] if r["provider_id"] == "ignored")
        self.assertEqual(old["runtime_catalog_state"], "shadowed_by_shiju")
        self.assertEqual(ignored["runtime_catalog_state"], "overlay_provider_not_loaded")

    def test_source_hash_matches_bytes(self):
        self.assertEqual(self.inv["sources"][str(self.api)]["sha256"],
                         hashlib.sha256(self.api.read_bytes()).hexdigest())

    def test_groups_are_disjoint_complete_and_record_catalog_drift(self):
        groups = build_groups(self.inv, self.seed)
        assigned = [p for g in groups["groups"] for p in g["providers"]]
        self.assertEqual(sorted(assigned), ["a", "b", "ignored", "shiju"])
        self.assertEqual(groups["added_providers"], ["ignored"])
        self.assertEqual(groups["removed_providers"], ["removed"])
        self.assertEqual(sum(g["model_count"] for g in groups["groups"]), 5)

    def test_duplicate_group_assignment_is_rejected(self):
        self.seed["research_groups"][1]["providers"].append("a")
        with self.assertRaisesRegex(ValueError, "duplicate"):
            build_groups(self.inv, self.seed)

    def test_adapter_override_inherits_provider_api(self):
        catalog = json.loads(self.api.read_text())
        catalog["a"]["models"]["key"]["provider"] = {"adapter": "anthropic"}
        self.api.write_text(json.dumps(catalog))
        row = build_inventory([self.api, self.shiju])["records"][0]
        self.assertEqual(row["effective_declarations"]["api"], "https://a/v1")
        self.assertEqual(row["effective_declarations"]["adapter"], "anthropic")
        self.assertIn("npm_adapter_precedence_requires_review", row["metadata_gaps"])

    def test_duplicate_source_json_keys_are_rejected(self):
        self.api.write_text('{"same": {}, "same": {}}')
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            load_json(self.api)

    def test_valid_official_and_live_claims_pass_contract(self):
        for conclusion, evidence_type in (("官方确认", "official_docs"), ("实测通过", "real_call")):
            with self.subTest(conclusion=conclusion):
                report = self.report_fixture()
                # Keep an official_query for the other insufficient records.
                report["evidence"].append({**report["evidence"][0], "id": "confirmed", "type": evidence_type})
                row = report["records"][0]
                row.update(conclusion=conclusion, evidence_ids=["confirmed"],
                           request_protocols=["chat"], endpoints=["https://example.com/v1"],
                           live_verification={"http_status": 200, "binding": {"model": "deployment"},
                                              "events": ["delta", "done"], "final_state": "completed"})
                self.assertEqual(check_research(self.inv, report, 1), [])

    def test_cli_reports_exit_codes_and_reproducible_inventory(self):
        root = Path(self.temp.name)
        script = Path(__file__).with_name("audit_model_protocols.py")
        seed = root / "seed.json"
        seed.write_text(json.dumps(self.seed))
        output = root / "out"
        cmd = [sys.executable, str(script), "inventory", "--api", str(self.api),
               "--shiju", str(self.shiju), "--output", str(output), "--baseline", str(seed)]
        first = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(first.returncode, 0, first.stderr)
        before = {p.name: p.read_bytes() for p in output.iterdir()}
        # Default regeneration uses the generated group artifact, independent of the plan file.
        second = subprocess.run(cmd[:-2], capture_output=True, text=True)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(before, {p.name: p.read_bytes() for p in output.iterdir()})
        report = self.report_fixture()
        report_path = root / "report.json"
        report_path.write_text(json.dumps(report))
        check = [sys.executable, str(script), "check-research", "--inventory",
                 str(output / "catalog-inventory.json"), "--report", str(report_path), "--group", "1"]
        result = subprocess.run(check, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("records=3", result.stdout)
        report["records"].pop()
        report_path.write_text(json.dumps(report))
        result = subprocess.run(check, capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn("coverage mismatch", result.stderr)

    def matrix_fixture(self):
        return {"schema_version": 1, "checked_at": "2026-11-12",
                "source_hashes": {s: v["sha256"] for s, v in self.inv["sources"].items()},
                "records": [{**{k: r[k] for k in ("source", "provider_id", "model_key",
                    "model_api_id", "canonical_model_id")}, "source_declarations": r["effective_declarations"],
                    "capabilities": r["capabilities"], "evidence_type": "offline_selector",
                    "protocols": {p: [{"status": "rejected", "reasons": ["UnknownProtocols"]}]
                                  for p in ("chat", "messages", "responses", "gemini")}}
                            for r in self.inv["records"]]}

    def test_coverage_valid_matrix_and_cli(self):
        from audit_model_protocols import check_coverage
        matrix = self.matrix_fixture()
        self.assertEqual(check_coverage(self.inv, matrix), [])
        root = Path(self.temp.name)
        inv, path = root / "inventory.json", root / "matrix.json"
        inv.write_text(json.dumps(self.inv))
        path.write_text(json.dumps(matrix))
        result = subprocess.run([sys.executable, str(Path(__file__).with_name("audit_model_protocols.py")),
            "check-coverage", "--inventory", str(inv), "--matrix", str(path)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("protocol_cells=20", result.stdout)

    def test_coverage_rejects_inventory_omission_and_unbound_success(self):
        from audit_model_protocols import check_coverage
        matrix = self.matrix_fixture()
        self.inv["records"].pop()
        matrix["records"].pop()
        self.assertTrue(check_coverage(self.inv, matrix))
        self.inv = build_inventory([self.api, self.shiju])
        matrix = self.matrix_fixture()
        row = matrix["records"][0]
        row["candidate_endpoints"] = [{"protocol": "chat", "url": "https://a/v1/chat/completions",
                                        "deployment": "a"}]
        row["protocols"]["chat"] = [{"status": "native", "target": "chat",
            "endpoint": "https://a/v1/chat/completions", "deployment": "a"}]
        self.assertEqual(check_coverage(self.inv, matrix), [])
        row["protocols"]["chat"][0]["target"] = "messages"
        self.assertTrue(check_coverage(self.inv, matrix))

    def test_coverage_rejects_missing_duplicate_stale_and_unknown(self):
        from audit_model_protocols import check_coverage
        for mutation in ("missing", "duplicate", "hash", "status", "protocol", "identity", "capabilities", "evidence"):
            with self.subTest(mutation=mutation):
                matrix = self.matrix_fixture()
                row = matrix["records"][0]
                if mutation == "missing": matrix["records"].pop()
                elif mutation == "duplicate": matrix["records"].append(copy.deepcopy(row))
                elif mutation == "hash": matrix["source_hashes"][str(self.api)] = "old"
                elif mutation == "status": row["protocols"]["chat"][0]["status"] = "success"
                elif mutation == "protocol": del row["protocols"]["chat"]
                elif mutation == "identity": row["model_api_id"] = "wrong"
                elif mutation == "capabilities": row["capabilities"] = {}
                else: row["evidence_type"] = "real_call"
                self.assertTrue(check_coverage(self.inv, matrix))

    def test_coverage_rejects_changed_source_bytes_and_invalid_rejection(self):
        from audit_model_protocols import check_coverage
        matrix = self.matrix_fixture()
        matrix["records"][0]["protocols"]["chat"][0]["reasons"] = []
        self.assertTrue(check_coverage(self.inv, matrix))
        matrix = self.matrix_fixture()
        self.api.write_text("{}")
        self.assertTrue(check_coverage(self.inv, matrix))

    def report_fixture(self):
        self.inv["research_groups"] = build_groups(self.inv, self.seed)["groups"]
        rows = [r for r in self.inv["records"] if r["provider_id"] in ("a", "shiju")]
        report = {"schema_version": 1, "group": 1,
                  "source_hashes": {s: v["sha256"] for s, v in self.inv["sources"].items()},
                  "providers": ["a", "shiju"],
                  "evidence": [{"id": "attempt", "type": "official_query",
                                "url": "https://example.com/docs", "checked_at": "2026-11-11",
                                "excerpt": "No deployment-specific protocol documentation located."}],
                  "records": []}
        for row in rows:
            report["records"].append({
                **{k: row[k] for k in ("source", "provider_id", "model_key",
                                       "model_api_id", "canonical_model_id")},
                "conclusion": "资料不足", "evidence_ids": ["attempt"],
                "reason": "Missing deployment and protocol evidence; endpoint not inferred.",
                "request_protocols": [], "endpoints": [], "auth": "unknown",
                "deployment": "region and plan unknown",
                "capabilities": {k: "unknown" for k in (
                    "text", "tools", "thinking", "media", "streaming", "structured_output")},
                "limitations": ["Needs official SDK or deployment documentation"]})
        return report

    def test_insufficient_research_with_traceable_attempt_is_valid(self):
        self.assertEqual(check_research(self.inv, self.report_fixture(), 1), [])

    def test_missing_model_coverage_is_rejected(self):
        report = self.report_fixture()
        report["records"].pop()
        self.assertTrue(check_research(self.inv, report, 1))

    def test_source_hash_conflict_is_rejected(self):
        report = self.report_fixture()
        report["source_hashes"][str(self.api)] = "wrong"
        self.assertTrue(check_research(self.inv, report, 1))

    def test_model_identity_conflict_is_rejected(self):
        report = self.report_fixture()
        report["records"][0]["model_api_id"] = "wrong"
        self.assertTrue(check_research(self.inv, report, 1))

    def test_duplicate_and_out_of_group_rows_are_rejected(self):
        report = self.report_fixture()
        report["records"].append(copy.deepcopy(report["records"][0]))
        self.assertTrue(check_research(self.inv, report, 1))
        report["records"][-1]["provider_id"] = "b"
        self.assertTrue(check_research(self.inv, report, 1))

    def test_insufficient_research_without_attempt_or_reason_is_rejected(self):
        report = self.report_fixture()
        report["records"][0]["evidence_ids"] = []
        report["records"][0]["reason"] = ""
        self.assertTrue(check_research(self.inv, report, 1))

    def test_official_or_live_success_requires_matching_evidence(self):
        for conclusion in ("官方确认", "实测通过"):
            with self.subTest(conclusion=conclusion):
                report = self.report_fixture()
                report["records"][0]["conclusion"] = conclusion
                self.assertTrue(check_research(self.inv, report, 1))

    def test_invalid_date_and_dangling_evidence_are_rejected(self):
        report = self.report_fixture()
        report["evidence"][0]["checked_at"] = "2026-02-30"
        self.assertTrue(check_research(self.inv, report, 1))
        report = self.report_fixture()
        report["records"][0]["evidence_ids"] = ["missing"]
        self.assertTrue(check_research(self.inv, report, 1))


if __name__ == "__main__":
    unittest.main()
