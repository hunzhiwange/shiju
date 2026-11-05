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
    def test_opencode_builtin_catalog_is_rejected_before_activation(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.opencode_native = True
            with self.assertRaisesRegex(ValueError, "unselected native skill"):
                fixture.response({"messages": [{"role": "system", "content":
                    "<available_skills><skill><name>" + fixture.name +
                    "</name></skill><skill><name>customize-opencode</name></skill></available_skills>"}],
                    "tools": [{"function": {"name": "skill"}}]})
            self.assertEqual(fixture.snapshot()["native_skill_invocations"], 0)

    def test_kiro_native_catalog_activates_disclose_context_with_name(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.kiro_native = True
            body = {"messages": [{"role": "user", "content": "Use selected skill"}],
                    "tools": [{"function": {"name": "disclose_context",
                        "description": 'name: "' + fixture.name + '" - fixture',
                        "parameters": {"required": ["name"]}}}]}
            message, finish = fixture.response(body)
            call = message["tool_calls"][0]["function"]
            self.assertEqual((call["name"], finish), ("disclose_context", "tool_calls"))
            self.assertEqual(json.loads(call["arguments"]), {"name": fixture.name})
            self.assertEqual(fixture.snapshot()["native_skill_invocations"], 1)
            self.assertFalse(fixture.snapshot()["skill_body_read"])

    def test_kiro_unselected_catalog_is_rejected_before_activation(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.kiro_native = True
            with self.assertRaisesRegex(ValueError, "catalog differs"):
                fixture.response({"messages": [], "tools": [{"name": "disclose_context",
                    "description": 'name: "' + fixture.name + '" - selected\nname: "unselected" - forbidden'}]})
            self.assertEqual(fixture.snapshot()["native_skill_invocations"], 0)

    def test_kiro_empty_catalog_does_not_reuse_old_prompt_as_discovery(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.kiro_native = True
            fixture.mode = "absent"
            message, finish = fixture.response({"messages": [{"role": "user",
                "content": 'name: "' + fixture.name + '" - old history'}], "tools": []})
            self.assertEqual((message["content"], finish), ("SKILL_ABSENT", "stop"))
            self.assertEqual(fixture.snapshot()["discovery_events"], 0)

    def test_kilocode_exact_system_catalog_activates_native_name_argument(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.kilocode_native = True
            body = {"messages": [{"role": "system", "content": "<available_skills><skill><name>" + fixture.name + "</name></skill></available_skills>"}],
                    "tools": [{"function": {"name": "skill",
                        "parameters": {"properties": {"name": {"type": "string"}}}}}]}
            message, finish = fixture.response(body)
            self.assertEqual(finish, "tool_calls")
            self.assertEqual(json.loads(message["tool_calls"][0]["function"]["arguments"]), {"name": fixture.name})
            self.assertFalse(fixture.snapshot()["skill_body_read"])
            self.assertFalse(fixture.snapshot()["attachment_read"])

    def test_kilocode_builtin_catalog_is_rejected_before_native_activation(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.kilocode_native = True
            with self.assertRaisesRegex(ValueError, "unselected native skill"):
                fixture.response({"messages": [{"role": "system", "content":
                    "<available_skills><skill><name>" + fixture.name + "</name></skill><skill><name>kilo-config</name></skill></available_skills>"}], "tools": [{"function": {"name": "skill"}}]})
            self.assertEqual(fixture.snapshot()["native_skill_invocations"], 0)

    def test_kilocode_empty_catalog_cannot_reuse_history_as_discovery(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.kilocode_native = True
            fixture.mode = "absent"
            message, finish = fixture.response({"messages": [{"role": "user", "content": "<available_skills><skill><name>" + fixture.name + "</name></skill></available_skills>"}],
                "tools": [{"function": {"name": "read"}}]})
            self.assertEqual((message["content"], finish), ("SKILL_ABSENT", "stop"))
            self.assertEqual(fixture.snapshot()["discovery_events"], 0)

    def test_copilot_exact_native_catalog_activates_with_skill_argument(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.copilot_native = True
            body = {"messages": [{"role": "system", "content": "<available_skills><skill><name>" + fixture.name + "</name></skill></available_skills>"}],
                    "tools": [{"function": {"name": "skill", "parameters": {"properties": {"skill": {"type": "string"}}}}}]}
            message, finish = fixture.response(body)
            self.assertEqual(finish, "tool_calls")
            call = message["tool_calls"][0]["function"]
            self.assertEqual(call["name"], "skill")
            self.assertEqual(json.loads(call["arguments"]), {"skill": fixture.name})
            self.assertFalse(fixture.snapshot()["skill_body_read"])
            self.assertFalse(fixture.snapshot()["attachment_read"])

    def test_copilot_native_acknowledgement_and_disk_expansion_are_both_required(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.copilot_native = True
            view = Path(self.advertised(fixture)[0]["content"].split()[-1]).parent
            messages = [{"role": "system", "content": "<available_skills><skill><name>" + fixture.name + "</name></skill></available_skills>"}]
            tools = [{"function": {"name": name, "parameters": {"properties": {field: {"type": "string"}}, "required": [field]}}}
                     for name, field in (("skill", "skill"), ("view", "path"))]
            message, _ = fixture.response({"messages": messages, "tools": tools})
            call = message["tool_calls"][0]
            expansion = {"role": "user", "content": "Base directory for this skill: " + str(view) + "\n" + fixture.skill.read_text()}
            with self.assertRaisesRegex(ValueError, "mounted view"):
                fixture.response({"messages": messages + [expansion], "tools": tools})
            messages += [{"role": "tool", "tool_call_id": call["id"], "content": 'Skill "' + fixture.name + '" loaded successfully. Follow the instructions in the skill context.'}, expansion]
            message, _ = fixture.response({"messages": messages, "tools": tools})
            read = message["tool_calls"][0]["function"]
            self.assertEqual(read["name"], "view")
            self.assertEqual(Path(json.loads(read["arguments"])["path"]), view / "references/value.txt")
            self.assertTrue(fixture.snapshot()["skill_body_read"])
            self.assertFalse(fixture.snapshot()["attachment_read"])

    def test_copilot_ambient_or_builtin_skill_never_passes_catalog_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.copilot_native = True
            with self.assertRaisesRegex(ValueError, "Copilot native catalog"):
                fixture.response({"messages": [{"role": "system", "content":
                    "<available_skills><skill><name>" + fixture.name + "</name></skill><skill><name>ambient</name></skill></available_skills>"}],
                    "tools": [{"function": {"name": "skill"}}]})
            self.assertEqual(fixture.snapshot()["native_skill_invocations"], 0)

    def test_copilot_disabled_catalog_and_utility_requests_do_not_forge_discovery(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.copilot_native = True
            fixture.response({"messages": [], "tools": []})
            self.assertEqual(fixture.snapshot()["model_requests"], 0)
            fixture.mode = "absent"
            message, finish = fixture.response({"messages": [{"role": "system", "content": "No skills"}],
                                               "tools": [{"function": {"name": "view"}}]})
            self.assertEqual((message["content"], finish), ("SKILL_ABSENT", "stop"))
            self.assertEqual(fixture.snapshot()["discovery_events"], 0)

    def test_vibe_requires_exact_native_catalog_and_name_argument(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.vibe_native = True
            tools = [{"function": {"name": "skill", "parameters": {"properties": {"name": {"type": "string"}}}}}]
            body = {"messages": [{"role": "user", "content": fixture.prompt}], "tools": tools}
            with self.assertRaisesRegex(ValueError, "Vibe native catalog"):
                fixture.response(body)
            fixture.phase_first = True
            body["messages"].insert(0, {"role": "system", "content": "<skill><name>" + fixture.name + "</name></skill>"})
            message, finish = fixture.response(body)
            self.assertEqual(finish, "tool_calls")
            self.assertEqual(message["tool_calls"][0]["function"]["name"], "skill")
            self.assertEqual(json.loads(message["tool_calls"][0]["function"]["arguments"]), {"name": fixture.name})
            self.assertFalse(fixture.snapshot()["attachment_read"])

    def test_vibe_rejects_ambient_catalog_and_ignores_utility_requests(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.vibe_native = True
            fixture.response({"messages": [], "tools": []})
            self.assertEqual(fixture.snapshot()["model_requests"], 0)
            self.assertTrue(fixture.phase_first)
            with self.assertRaisesRegex(ValueError, "Vibe native catalog"):
                fixture.response({"messages": [{"role": "system", "content": "<skill><name>unselected</name></skill>"}],
                                  "tools": [{"function": {"name": "skill"}}]})
            self.assertEqual(fixture.snapshot()["native_skill_invocations"], 0)

    def test_vibe_removed_skill_is_absent_despite_old_user_history(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.vibe_native = True
            fixture.mode = "absent"
            message, finish = fixture.response({"messages": self.advertised(fixture),
                "tools": [{"function": {"name": "read_file"}}]})
            self.assertEqual((message["content"], finish), ("SKILL_ABSENT", "stop"))
            self.assertTrue(fixture.discovery_observations[-1]["native_activation_unregistered"])
            self.assertEqual(fixture.snapshot()["discovery_events"], 0)

    def test_goose_requires_native_catalog_and_exact_activation_schema(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.goose_native = True
            tools = [{"type": "function", "function": {"name": "load_skill",
                      "parameters": {"properties": {"name": {"type": "string"}}}}}]
            with self.assertRaises(ValueError):
                fixture.response({"messages": [{"role": "user", "content": fixture.prompt}], "tools": tools})
            fixture.phase_first = True
            message, finish = fixture.response({"messages": [
                {"role": "system", "content": "# Skills\n\nYou have these skills at your disposal.\n- " + fixture.name + ": fixture"},
                {"role": "user", "content": fixture.prompt}], "tools": tools})
            self.assertEqual(finish, "tool_calls")
            call = message["tool_calls"][0]["function"]
            self.assertEqual(call["name"], "load_skill")
            self.assertEqual(json.loads(call["arguments"]), {"name": fixture.name})
            self.assertEqual(fixture.snapshot()["native_skill_invocations"], 1)
            self.assertFalse(fixture.snapshot()["attachment_read"])

    def test_codebuddy_preflight_does_not_count_as_skill_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.codebuddy_native = True
            fixture.response({"messages": [{"role": "user", "content": "preflight"}], "tools": []})
            self.assertEqual(fixture.snapshot()["model_requests"], 0)
            self.assertEqual(fixture.snapshot()["discovery_events"], 0)
            self.assertTrue(fixture.phase_first)

    def test_codebuddy_tool_catalog_activates_unqualified_native_skill(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.codebuddy_native = True
            body = {"messages": [{"role": "user", "content": fixture.prompt}], "tools": [
                {"type": "function", "function": {"name": "Skill", "description": "<available_skills>\n- " + fixture.name + ": fixture\n</available_skills>",
                    "parameters": {"properties": {"skill": {"type": "string"}}}}}]}
            message, finish = fixture.response(body)
            self.assertEqual(finish, "tool_calls")
            call = message["tool_calls"][0]["function"]
            self.assertEqual(call["name"], "Skill")
            self.assertEqual(json.loads(call["arguments"]), {"skill": fixture.name})
            self.assertTrue(fixture.snapshot()["first_prompt_clean"])
            self.assertFalse(fixture.snapshot()["attachment_read"])

    def test_codebuddy_catalog_rejects_unselected_builtin_or_plugin(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.codebuddy_native = True
            body = {"messages": [{"role": "user", "content": fixture.prompt}], "tools": [
                {"function": {"name": "Skill", "description": "<available_skills>\n- " + fixture.name + ": selected\n- loop: unselected builtin\n</available_skills>",
                    "parameters": {"properties": {"skill": {"type": "string"}}}}}]}
            with self.assertRaisesRegex(ValueError, "unselected native skill leaked"):
                fixture.response(body)
            self.assertEqual(fixture.snapshot()["native_skill_invocations"], 0)
            self.assertEqual(fixture.snapshot()["discovery_events"], 0)

    def test_codebuddy_empty_selection_uses_only_native_skill_descriptor(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.codebuddy_native = True
            fixture.mode = "absent"
            body = {"messages": [{"role": "user", "content": fixture.prompt}], "tools": [
                {"function": {"name": "Read", "description": "Example: <available_skills>\n- parameter: unrelated documentation\n</available_skills>",
                    "parameters": {"properties": {"file_path": {"type": "string"}}}}}]}
            message, finish = fixture.response(body)
            self.assertEqual((message["content"], finish), ("SKILL_ABSENT", "stop"))
            self.assertTrue(fixture.discovery_observations[-1]["native_activation_unregistered"])
            self.assertEqual(fixture.snapshot()["native_skill_invocations"], 0)

    def test_gemini_wire_requires_correlated_native_activation(self):
        from acp_skill_mount_gemini import generate_message, generate_wire
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.gemini_native = True
            view = fixture.root / "view" / fixture.name
            view.parent.mkdir()
            view.symlink_to(fixture.directory, target_is_directory=True)
            body = {"systemInstruction": {"parts": [{"text": "<available_skills>" + fixture.name + "<location>" + str(view / "SKILL.md") + "</location></available_skills>"}]},
                    "contents": [{"role": "user", "parts": [{"text": fixture.prompt}]}],
                    "tools": [{"functionDeclarations": [
                        {"name": "activate_skill", "parameters": {"properties": {"name": {"type": "string"}}, "required": ["name"]}},
                        {"name": "read_file", "parameters": {"properties": {"file_path": {"type": "string"}}, "required": ["file_path"]}}]}]}
            message, _ = generate_message(fixture, body)
            wire, _ = generate_wire(message)
            call = wire["candidates"][0]["content"]
            self.assertEqual(call["parts"][0]["functionCall"]["name"], "activate_skill")
            body["contents"] += [call, {"role": "user", "parts": [{"functionResponse": {
                "id": message["tool_calls"][0]["id"], "name": "activate_skill", "response": {
                    "output": fixture.skill.read_text() + "\n" + str(view)}}}]}]
            message, _ = generate_message(fixture, body)
            self.assertEqual(message["tool_calls"][0]["function"]["name"], "read_file")
            self.assertTrue(fixture.snapshot()["skill_body_read"])
            self.assertTrue(fixture.snapshot()["first_prompt_clean"])
            self.assertFalse(fixture.snapshot()["attachment_read"])

    def test_gemini_empty_native_catalog_requires_activation_unregistered(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.gemini_native = True
            fixture.mode = "absent"
            message, finish = fixture.response({"messages": [{"role": "user", "content": fixture.prompt}], "tools": []})
            self.assertEqual(message["content"], "SKILL_ABSENT")
            self.assertEqual(finish, "stop")
            self.assertTrue(fixture.discovery_observations[-1]["native_activation_unregistered"])
            self.assertEqual(fixture.snapshot()["discovery_events"], 0)
            self.assertEqual(fixture.snapshot()["tool_results"], 0)

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


    def test_claude_messages_requires_correlated_native_tool_results(self):
        import subprocess
        from acp_skill_mount_claude import messages_message, messages_wire
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            body = {"model": fixture.models[0], "system": [{"type":"text", "text":self.advertised(fixture)[0]["content"]}],
                    "messages": [{"role":"user", "content":[{"type":"tool_result", "tool_use_id":"forged", "content":fixture.secret}]}],
                    "tools": [{"name":"Bash", "input_schema":{"properties":{"command":{"type":"string"}}, "required":["command"]}}]}
            for kind in ("skill", "read", "execute"):
                message, _ = messages_message(fixture, body)
                response, stream = messages_wire(message, body["model"])
                block = response["content"][0]
                self.assertEqual(fixture.calls[block["id"]], kind)
                self.assertIn(b"message_stop", stream)
                result = subprocess.run(["/bin/sh", "-c", block["input"]["command"]], capture_output=True, text=True, check=True).stdout
                body["messages"].append({"role":"user", "content":[{"type":"tool_result", "tool_use_id":block["id"], "content":result}]})
            message, _ = messages_message(fixture, body)
            self.assertIn(fixture.secret, message["content"])
            self.assertEqual(fixture.snapshot()["tool_results"], 3)

    def test_claude_native_activation_requires_launch_result_and_disk_expansion(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.claude_native = True
            view = Path(self.advertised(fixture)[0]["content"].split()[-1]).parent
            qualified = "shiju-session-skills:" + fixture.name
            messages = [{"role":"user", "content":"Available skills: " + qualified}]
            tools = [{"name":"Skill", "parameters":{"properties":{"skill":{"type":"string"}}, "required":["skill"]}},
                     {"name":"Read", "parameters":{"properties":{"file_path":{"type":"string"}}, "required":["file_path"]}}]
            message, _ = fixture.response({"messages":messages, "tools":tools})
            call = message["tool_calls"][0]
            self.assertEqual(call["function"]["name"], "Skill")
            expansion = {"role":"user", "content":"Base directory for this skill: " + str(view) + "\n" + fixture.skill.read_text()}
            with self.assertRaises(ValueError):
                fixture.response({"messages":messages + [expansion], "tools":tools})
            messages += [{"role":"tool", "tool_call_id":call["id"], "content":"Launching skill: " + qualified}, expansion]
            message, _ = fixture.response({"messages":messages, "tools":tools})
            self.assertEqual(message["tool_calls"][0]["function"]["name"], "Read")
            self.assertTrue(fixture.snapshot()["skill_body_read"])

    def test_claude_messages_http_accepts_native_auth_and_beta_query(self):
        from urllib.request import Request, urlopen
        with tempfile.TemporaryDirectory() as directory, SkillMountFixture(Path(directory)) as fixture:
            body = {"model":fixture.models[0], "messages":self.advertised(fixture), "tools":[
                {"name":"Read", "input_schema":{"properties":{"file_path":{"type":"string"}}, "required":["file_path"]}}]}
            request = Request(fixture.url + "/v1/messages?beta=true", data=json.dumps(body).encode(),
                              headers={"x-api-key":fixture.key, "Content-Type":"application/json"})
            with urlopen(request) as response:
                message = json.load(response)
            self.assertEqual(message["content"][0]["type"], "tool_use")
            self.assertEqual(fixture.snapshot()["model_requests"], 1)
            self.assertEqual(fixture.snapshot()["errors"], 0)

    def test_claude_removed_skill_requires_native_rejection_despite_historical_catalog(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.claude_native = True
            fixture.mode = "absent"
            qualified = "shiju-session-skills:" + fixture.name
            messages = [{"role":"user", "content":"Historical available skills: " + qualified}]
            tools = [{"name":"Skill", "parameters":{"properties":{"skill":{}}, "required":["skill"]}}]
            message, _ = fixture.response({"messages":messages, "tools":tools})
            call = message["tool_calls"][0]
            with self.assertRaises(ValueError):
                fixture.response({"messages":messages + [{"role":"tool", "tool_call_id":call["id"], "content":"Launching skill: " + qualified}], "tools":tools})
            message, finish = fixture.response({"messages":messages + [{"role":"tool", "tool_call_id":call["id"], "content":"Unknown skill: " + qualified}], "tools":tools})
            self.assertEqual(finish, "stop")
            self.assertEqual(message["content"], "SKILL_ABSENT")
            self.assertTrue(fixture.snapshot()["discovery_observations"][-1]["native_rejection_verified"])

    def test_claude_added_skill_requires_actual_native_expansion(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.claude_native = True
            fixture.mode = "added"
            qualified = "shiju-session-skills:" + fixture.name
            messages = [{"role":"user", "content":"Historical available skills: " + qualified}]
            tools = [{"name":"Skill", "parameters":{"properties":{"skill":{}}, "required":["skill"]}}]
            message, _ = fixture.response({"messages":messages, "tools":tools})
            call = message["tool_calls"][0]
            self.assertEqual(json.loads(call["function"]["arguments"])["skill"], qualified + "-added")
            with self.assertRaises(ValueError):
                fixture.response({"messages":messages + [{"role":"tool", "tool_call_id":call["id"], "content":"Launching skill: " + qualified + "-added"}], "tools":tools})
            expansion = {"role":"user", "content":"Base directory for this skill: /owned/view/added\nADDED_SKILL_BODY"}
            message, _ = fixture.response({"messages":messages + [{"role":"tool", "tool_call_id":call["id"], "content":"Launching skill: " + qualified + "-added"}, expansion], "tools":tools})
            self.assertEqual(json.loads(message["tool_calls"][0]["function"]["arguments"])["skill"], qualified)
            self.assertTrue(fixture.snapshot()["discovery_observations"][-1]["added_native_verified"])

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


    def test_qwen_user_prompt_name_is_not_native_discovery(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.qwen_native = True
            with self.assertRaisesRegex(ValueError, "mounted view"):
                fixture.response({"messages": [{"role": "user", "content": fixture.prompt}], "tools": []})
            self.assertEqual(fixture.snapshot()["discovery_events"], 0)

    def test_qwen_native_catalog_requires_real_skill_result_with_view_path(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.qwen_native = True
            body = {"messages": [{"role": "user", "content": "<available_skills>" + fixture.name + "</available_skills>"}],
                    "tools": [{"type": "function", "function": {"name": "skill", "parameters": {
                        "properties": {"skill": {"type": "string"}}, "required": ["skill"]}}}]}
            message, finish = fixture.response(body)
            self.assertEqual(finish, "tool_calls")
            call = message["tool_calls"][0]
            self.assertEqual(call["function"]["name"], "skill")
            self.assertEqual(json.loads(call["function"]["arguments"]), {"skill": fixture.name})
            body["messages"].append({"role": "tool", "tool_call_id": call["id"], "content": "unregistered"})
            with self.assertRaisesRegex(ValueError, "mounted view"):
                fixture.response(body)
            self.assertFalse(accepted(fixture.snapshot(), {}, 1, 0))


    def test_qwen_json_native_result_is_decoded_before_requesting_attachment(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = SkillMountFixture(Path(directory))
            fixture.qwen_native = True
            view = Path(self.advertised(fixture)[0]["content"].split()[-1]).parent
            body = {"messages": [{"role": "user", "content": "<available_skills>" + fixture.name + "</available_skills>"}],
                    "tools": [{"type": "function", "function": {"name": "skill", "parameters": {
                        "properties": {"skill": {"type": "string"}}, "required": ["skill"]}}},
                        {"type": "function", "function": {"name": "read_file", "parameters": {
                        "properties": {"absolute_path": {"type": "string"}}, "required": ["absolute_path"]}}}]}
            message, _ = fixture.response(body)
            call = message["tool_calls"][0]
            content = "Base directory for this skill: " + str(view) + "\n" + fixture.skill.read_text().split("---\n")[-1]
            body["messages"].append({"role": "tool", "tool_call_id": call["id"],
                                     "content": json.dumps({"output": [{"text": content}]})})
            message, _ = fixture.response(body)
            self.assertEqual(message["tool_calls"][0]["function"]["name"], "read_file")
            self.assertTrue(fixture.snapshot()["skill_body_read"])


if __name__ == "__main__":
    unittest.main()
