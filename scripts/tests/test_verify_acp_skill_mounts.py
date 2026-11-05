"""Behavior tests for the skill-specific acceptance entry (never supplier evidence)."""
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from acp_skill_mount_fixture import SkillMountFixture
from verify_acp_skill_mounts import accepted, select_entry


class SkillMountHarnessTests(unittest.TestCase):
    def advertised(self, fixture):
        # A real symlink view is used only to test the HTTP fixture state machine.
        # This is never an agent acceptance report.
        view = fixture.root / "test-view" / fixture.name
        view.parent.mkdir(exist_ok=True)
        view.symlink_to(fixture.directory, target_is_directory=True)
        return [{"role": "user", "content": fixture.prompt + " " + str(view / "SKILL.md")}]

    def test_responses_requires_native_discovery_and_correlated_file_tools(self):
        import subprocess
        from acp_skill_mount_codex import responses_message, responses_wire
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            metadata = self.advertised(fixture)[0]["content"]
            body = {"model": "acp-fixture-primary", "input": [{"role":"developer", "content": metadata}],
                    "tools": [{"type":"function", "name":"exec_command", "parameters": {
                        "properties":{"cmd":{"type":"string"}}, "required":["cmd"]}}]}
            for kind in ("skill", "read", "execute"):
                message, finish = responses_message(fixture, body)
                self.assertEqual(finish, "tool_calls")
                call = message["tool_calls"][0]
                self.assertEqual(fixture.calls[call["id"]], kind)
                response, stream = responses_wire(message, body["model"])
                self.assertEqual(response["output"][0]["call_id"], call["id"])
                self.assertIn(b"response.completed", stream)
                result = subprocess.run(["/bin/sh", "-c", json.loads(call["function"]["arguments"])["cmd"]],
                                        capture_output=True, text=True, check=True).stdout
                body["input"].append({"type":"function_call_output", "call_id":call["id"], "output":result})
            message, finish = responses_message(fixture, body)
            self.assertEqual(finish, "stop")
            self.assertIn(fixture.secret, message["content"])
            self.assertTrue(fixture.snapshot()["first_prompt_clean"])
            self.assertEqual(fixture.snapshot()["tool_results"], 3)


    def test_unknown_and_duplicate_agents_fail(self):
        for entries in ([], [{"key": "codex"}, {"key": "codex"}]):
            with self.assertRaises(ValueError):
                select_entry({"entries": entries}, "codex")

    def test_zero_requests_and_forged_flow_fail(self):
        self.assertFalse(accepted({"model_requests": 0}, {"new": True}, 1, 0))
        self.assertFalse(accepted({}, {}, 0, 0))

    def test_random_value_is_only_in_attachment(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            self.assertNotIn(fixture.secret, fixture.prompt)
            self.assertNotIn(fixture.secret, fixture.skill.read_text())
            self.assertEqual(fixture.attachment.read_text(), fixture.secret)
            self.assertNotIn(fixture.secret, json.dumps(fixture.snapshot()))

    def test_answer_requires_read_and_execution_results(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            with self.assertRaises(ValueError):
                fixture.answer([])
            with self.assertRaises(ValueError):
                fixture.answer([{"role": "user", "content": fixture.secret}])
            with self.assertRaises(ValueError):
                fixture.answer([{"role": "tool", "tool_call_id": "invented", "content": fixture.secret}])

    def test_skill_body_is_read_before_attachment(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            message, _ = fixture.response({"messages": self.advertised(fixture),
                "tools": [{"type": "function", "function": {"name": "read_file", "parameters": {
                    "properties": {"path": {"type": "string"}}, "required": ["path"]}}}]})
            call = message["tool_calls"][0]
            self.assertEqual(Path(json.loads(call["function"]["arguments"])["path"]).resolve(), fixture.skill.resolve())

    def test_old_model_report_is_not_skill_evidence(self):
        self.assertFalse(accepted({"requests": [{"tool_results": 10}], "errors": 0},
                                  {"new": True, "restart": True}, 1, 0))


    def test_correlated_tools_use_real_files_and_script(self):
        import subprocess
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            messages = self.advertised(fixture)
            tools = [{"type": "function", "function": {"name": name, "parameters": {
                "properties": {field: {"type": "string"}}, "required": [field]}}}
                for name, field in (("read_file", "path"), ("exec_command", "command"))]
            for expected in ("skill", "read", "execute"):
                message, finish = fixture.response({"messages": messages, "tools": tools})
                self.assertEqual(finish, "tool_calls")
                call = message["tool_calls"][0]
                self.assertEqual(fixture.calls[call["id"]], expected)
                args = json.loads(call["function"]["arguments"])
                if "path" in args:
                    result = Path(args["path"]).read_text()
                else:
                    result = subprocess.run(["/bin/sh", str(fixture.script)], capture_output=True,
                                            text=True, check=True).stdout
                messages.extend([message, {"role": "tool", "tool_call_id": call["id"], "content": result}])
            message, finish = fixture.response({"messages": messages, "tools": tools})
            self.assertEqual(finish, "stop")
            self.assertIn(fixture.secret, message["content"])
            self.assertEqual(fixture.snapshot()["tool_results"], 3)
            self.assertTrue(fixture.snapshot()["script_executed"])

    def test_wrong_sentinel_cannot_unlock_reply(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.calls.update(skill="skill", read="read", execute="execute")
            messages = [{"role": "tool", "tool_call_id": key, "content": "WRONG_SENTINEL"}
                        for key in fixture.calls]
            with self.assertRaises(ValueError):
                fixture.answer(messages)

    def test_all_lifecycle_fields_and_one_actual_test_are_required(self):
        from acp_skill_mount_fixture import LIFECYCLE
        evidence = {"errors": 0, "model_requests": 4, "tool_requests": 3, "tool_results": 3,
                    "discovery_events": 1, "first_prompt_clean": True,
                    "attachment_read": True, "script_executed": True}
        flow = dict.fromkeys(LIFECYCLE, True)
        self.assertTrue(accepted(evidence, flow, 1, 0))
        for key in LIFECYCLE:
            self.assertFalse(accepted(evidence, dict(flow, **{key: False}), 1, 0))
        self.assertFalse(accepted(evidence, flow, 0, 0))
        self.assertFalse(accepted(evidence, flow, 1, 101))

    def test_fake_and_missing_cli_never_run_cargo(self):
        from argparse import Namespace
        from unittest.mock import patch
        import verify_acp_skill_mounts as harness
        for fake in (False, True):
            with self.subTest(fake=fake), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                binary = root / "fake-cli"
                if fake:
                    binary.write_text("#!/bin/sh\necho 1.0.0\n")
                    binary.chmod(0o700)
                identity = {"key": "codex", "local_command_candidates": [str(binary)],
                    "installed": {"command": [str(binary), "--version"]},
                    "distribution": {"release": "1.0.0"}, "underlying": {"kind": "same_distribution"},
                    "acp_launch": [str(binary)], "offline_acceptance": {"session-model": {
                        "config_root_env": "CODEX_HOME", "files": {"config.toml": "model = 'fixture'"}}}}
                inventory = root / "docs/shiju/acp-model-source/cli-versions.json"
                inventory.parent.mkdir(parents=True)
                inventory.write_text(json.dumps({"entries": [identity]}))
                manifest = root / "skills.json"
                manifest.write_text(json.dumps({"entries": [{"key": "codex", "version": "1.0.0", "evidence": []}]}))
                output = root / "report.json"
                with patch.object(harness, "ROOT", root), patch.object(harness, "cargo_slot", return_value=os.environ.get("CARGO_TARGET_DIR", str(harness.ROOT / "target/rust-slot-1"))), \
                     patch.object(harness, "execute_cargo") as cargo:
                    code = harness.run(Namespace(agent="codex", manifest=manifest, output=output))
                self.assertEqual(code, 1)
                self.assertFalse(json.loads(output.read_text())["passed"])
                cargo.assert_not_called()

    def test_http_fixture_is_authenticated_and_redacted(self):
        from urllib.request import Request, urlopen
        from urllib.error import HTTPError
        with tempfile.TemporaryDirectory() as directory, SkillMountFixture(Path(directory)) as fixture:
            with self.assertRaises(HTTPError):
                urlopen(fixture.url + "/_evidence")
            request = Request(fixture.url + "/_evidence", headers={"Authorization": "Bearer " + fixture.key})
            with urlopen(request) as response:
                snapshot = response.read().decode()
            self.assertNotIn(fixture.key, snapshot)
            self.assertNotIn(fixture.secret, snapshot)
            self.assertEqual(json.loads(snapshot)["model_requests"], 0)

    def test_http_zero_tools_is_an_error_with_one_actual_request(self):
        from urllib.request import Request, urlopen
        from urllib.error import HTTPError
        with tempfile.TemporaryDirectory() as directory, SkillMountFixture(Path(directory)) as fixture:
            body = json.dumps({"model": fixture.models[0], "messages": [{"role": "user", "content": fixture.prompt}], "tools": []}).encode()
            request = Request(fixture.url + "/v1/chat/completions", data=body,
                              headers={"Authorization": "Bearer " + fixture.key})
            with self.assertRaises(HTTPError):
                urlopen(request)
            self.assertEqual(fixture.snapshot()["model_requests"], 1)
            self.assertEqual(fixture.snapshot()["errors"], 1)
            self.assertFalse(fixture.snapshot()["script_executed"])

    def test_new_phase_rotates_attachment_and_rejects_old_tool_results(self):
        from urllib.request import Request, urlopen
        with tempfile.TemporaryDirectory() as directory, SkillMountFixture(Path(directory)) as fixture:
            old = fixture.secret
            fixture.calls["old"] = "read"
            request = Request(fixture.url + "/_begin", data=b'{"mode":"present"}',
                              headers={"Authorization": "Bearer " + fixture.key})
            with urlopen(request) as response:
                self.assertEqual(response.status, 204)
            self.assertNotEqual(fixture.secret, old)
            self.assertEqual(fixture.attachment.read_text(), fixture.secret)
            with self.assertRaises(ValueError):
                fixture.answer([{"role": "tool", "tool_call_id": "old", "content": old}])

    def test_original_source_is_not_a_native_mount_view(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            with self.assertRaisesRegex(ValueError, "mounted view"):
                fixture.response({"messages": [{"role": "user", "content": str(fixture.skill)}], "tools": []})
            self.assertEqual(fixture.snapshot()["discovery_events"], 0)
            self.assertEqual(fixture.snapshot()["model_requests"], 1)


if __name__ == "__main__":
    unittest.main()
