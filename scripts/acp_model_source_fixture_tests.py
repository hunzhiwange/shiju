"""Tool tests use HTTP clients, never claim to be a supplier ACP implementation."""
import json
import unittest
import urllib.error
import urllib.request

from acp_model_source_fixture import ModelFixture, successful


class HarnessContract(unittest.TestCase):
    def test_chat_responses_have_distinct_ids_and_stream_chunks_share_one_id(self):
        with ModelFixture() as fixture:
            response_ids = []
            for stream in (True, True, False):
                request = urllib.request.Request(fixture.url + "/v1/chat/completions",
                    data=json.dumps({"model": fixture.models[0], "stream": stream,
                        "messages": [{"role": "user", "content": "reply"}]}).encode(),
                    headers={"Authorization": "Bearer " + fixture.key, "Content-Type": "application/json"})
                with urllib.request.urlopen(request) as reply:
                    body = reply.read().decode()
                if stream:
                    chunks = [json.loads(line.removeprefix("data: ")) for line in body.splitlines()
                        if line.startswith("data: ") and line != "data: [DONE]"]
                    self.assertEqual(len(chunks), 2)
                    self.assertEqual(chunks[0]["id"], chunks[1]["id"])
                    response_ids.append(chunks[0]["id"])
                else:
                    response_ids.append(json.loads(body)["id"])
            self.assertEqual(len(set(response_ids)), 3)
            self.assertEqual(len(fixture.snapshot()["requests"]), 3)
            self.assertEqual(fixture.snapshot()["errors"], 0)

    def test_zero_requests_cannot_pass(self):
        self.assertFalse(successful({"requests": [], "errors": 0}, {}))

    def test_handshake_cannot_pass(self):
        self.assertFalse(successful({"handshake": True, "requests": [], "errors": 0}, {}))

    def test_forged_model_field_cannot_pass(self):
        self.assertFalse(successful({"model": "acp-fixture-primary", "errors": 0}, {}))

    def test_bad_auth_is_rejected_and_redacted(self):
        with ModelFixture() as fixture:
            req = urllib.request.Request(fixture.url + "/v1/chat/completions", b'{"model":"acp-fixture-primary"}',
                                         {"Authorization": "Bearer wrong", "Content-Type": "application/json"})
            with self.assertRaises(urllib.error.HTTPError):
                urllib.request.urlopen(req)
            evidence = fixture.snapshot()
            self.assertEqual(evidence, {"requests": [], "errors": 1, "error_kinds": {"auth": 1}})
            self.assertNotIn(fixture.key, json.dumps(evidence))

    def test_http_observations_are_independent_of_model_output(self):
        with ModelFixture() as fixture:
            req = urllib.request.Request(fixture.url + "/v1/chat/completions",
                                         json.dumps({"model": fixture.models[0], "messages": [{"role": "user", "content": "hello"}]}).encode(),
                                         {"Authorization": "Bearer " + fixture.key, "Content-Type": "application/json"})
            with urllib.request.urlopen(req) as reply:
                self.assertIn(fixture.challenge, reply.read().decode())
            evidence = fixture.snapshot()
            self.assertEqual(len(evidence["requests"]), 1)
            self.assertFalse(successful(evidence, {}))
            self.assertNotIn(fixture.key, json.dumps(evidence))


    def test_catalog_requires_model_auth_and_is_not_generation(self):
        with ModelFixture() as fixture:
            for path in ("/v1/models", "/v1/chat/completions/models", "/v1/messages/models"):
                request = urllib.request.Request(fixture.url + path, headers={
                    "Authorization": "Bearer " + fixture.key})
                with urllib.request.urlopen(request) as response:
                    self.assertEqual({item["id"] for item in json.load(response)["data"]}, set(fixture.models))
                request = urllib.request.Request(fixture.url + path, headers={
                    "Authorization": "Bearer " + fixture.control})
                with self.assertRaises(urllib.error.HTTPError):
                    urllib.request.urlopen(request)
            self.assertEqual(fixture.snapshot()["requests"], [])
            self.assertEqual(fixture.snapshot()["errors"], 3)

    def test_snow_native_read_uses_owned_absolute_challenge(self):
        with ModelFixture() as fixture:
            fixture.read_path = "/owned-fixture/challenge.txt"
            body = {"model": fixture.models[0], "messages": [{"role": "user", "content": "read"}],
                    "tools": [{"type": "function", "function": {"name": "filesystem-read", "parameters": {
                        "properties": {"filePath": {"type": "string"}}}}}]}
            request = urllib.request.Request(fixture.url + "/v1/chat/completions", json.dumps(body).encode(),
                {"Authorization": "Bearer " + fixture.key, "Content-Type": "application/json"})
            with urllib.request.urlopen(request) as response:
                call = json.load(response)["choices"][0]["message"]["tool_calls"][0]
            self.assertEqual(call["function"]["name"], "filesystem-read")
            self.assertEqual(json.loads(call["function"]["arguments"]), {"filePath": fixture.read_path})
            self.assertEqual(fixture.snapshot()["requests"][0]["tool_results"], 0)


class MessagesContract(unittest.TestCase):
    def test_messages_stream_auth_and_two_real_tool_results(self):
        with ModelFixture() as fixture:
            messages = [{"role": "user", "content": "read twice"}]
            for turn in range(3):
                body = {"model": fixture.models[0], "stream": True, "messages": messages,
                        "tools": [{"name": "Read", "input_schema": {"properties": {"file_path": {"type": "string"}}}}]}
                request = urllib.request.Request(fixture.url + "/v1/messages?beta=true", json.dumps(body).encode(),
                    {"x-api-key": fixture.key, "Content-Type": "application/json"})
                with urllib.request.urlopen(request) as response:
                    events = [json.loads(line[6:]) for line in response.read().decode().splitlines() if line.startswith("data: ")]
                self.assertEqual(events[0]["type"], "message_start")
                self.assertEqual(events[-1]["type"], "message_stop")
                block = events[1]["content_block"]
                if turn < 2:
                    self.assertEqual(block["type"], "tool_use")
                    arguments = json.loads(events[2]["delta"]["partial_json"])
                    self.assertEqual(arguments["file_path"], fixture.read_path)
                    messages.extend([{"role": "assistant", "content": [dict(block, input=arguments)]},
                                     {"role": "user", "content": [{"type": "tool_result", "tool_use_id": block["id"], "content": fixture.challenge}]}])
                else:
                    self.assertIn(fixture.challenge, events[2]["delta"]["text"])
            evidence = fixture.snapshot()
            self.assertEqual([r["tool_results"] for r in evidence["requests"]], [0, 1, 2])
            self.assertEqual(evidence["errors"], 0)
            self.assertNotIn(fixture.key, json.dumps(evidence))

    def test_messages_wrong_credentials_and_counting_are_not_generation(self):
        with ModelFixture() as fixture:
            body = json.dumps({"model": fixture.models[0]}).encode()
            request = urllib.request.Request(fixture.url + "/v1/messages/count_tokens", body,
                {"Authorization": "Bearer " + fixture.key, "Content-Type": "application/json"})
            with urllib.request.urlopen(request) as response:
                self.assertEqual(json.load(response), {"input_tokens": 1})
            self.assertEqual(fixture.snapshot(), {"requests": [], "errors": 0})
            request = urllib.request.Request(fixture.url + "/v1/messages", body,
                {"Authorization": "Bearer wrong", "Content-Type": "application/json"})
            with self.assertRaises(urllib.error.HTTPError):
                urllib.request.urlopen(request)
            self.assertEqual(fixture.snapshot(), {"requests": [], "errors": 1, "error_kinds": {"auth": 1}})


class RunnerContract(unittest.TestCase):
    def test_slot_must_be_absolute_and_reserved(self):
        from verify_acp_model_source import Unavailable, cargo_slot
        for value in ("", "target/rust-slot-1", "/tmp/arbitrary", "/tmp/rust-slot-11"):
            with self.assertRaises(Unavailable):
                cargo_slot({"CARGO_TARGET_DIR": value})

    def test_missing_cli_is_unavailable(self):
        from verify_acp_model_source import Unavailable, locate
        with self.assertRaisesRegex(Unavailable, "not installed"):
            locate({"local_command_candidates": ["nonexistent-acp-fixture-10"]}, {"PATH": ""})

    def test_no_implicit_package_install(self):
        from verify_acp_model_source import Unavailable, locate
        with self.assertRaises(Unavailable):
            locate({"local_command_candidates": ["npx", "npm", "uvx"]}, {})

    def test_no_unreviewed_native_recipe(self):
        import tempfile
        from pathlib import Path
        from verify_acp_model_source import Unavailable, isolated_environment
        with tempfile.TemporaryDirectory() as root, ModelFixture() as fixture:
            with self.assertRaisesRegex(Unavailable, "reviewed"):
                isolated_environment({}, Path(root), fixture, "cli-local")

    def test_native_recipe_rejects_home_production_and_traversal(self):
        import tempfile
        from pathlib import Path
        from verify_acp_model_source import Unavailable, isolated_environment
        recipes = [
            {"config_root_env": "HOME", "files": {"config": "x"}},
            {"config_root_env": "TEST_CONFIG_DIR", "files": {"config": "https://production.invalid/v1"}},
            {"config_root_env": "TEST_CONFIG_DIR", "files": {"../escape": "x"}},
        ]
        with tempfile.TemporaryDirectory() as root, ModelFixture() as fixture:
            for recipe in recipes:
                with self.assertRaises(Unavailable):
                    isolated_environment({"offline_acceptance": {"cli-local": recipe}}, Path(root), fixture, "cli-local")

    def test_real_fixture_tool_rounds_are_observed(self):
        with ModelFixture() as fixture:
            tool = {"type": "function", "function": {"name": "read_file", "parameters": {"properties": {"path": {"type": "string"}}}}}
            messages = [{"role": "user", "content": "read file"}]
            for round_number in range(3):
                body = {"model": fixture.models[0], "messages": messages, "tools": [tool]}
                req = urllib.request.Request(fixture.url + "/v1/chat/completions", json.dumps(body).encode(),
                                             {"Authorization": "Bearer " + fixture.key, "Content-Type": "application/json"})
                with urllib.request.urlopen(req) as reply:
                    response = json.load(reply)
                message = response["choices"][0]["message"]
                if round_number < 2:
                    messages.extend([message, {"role": "tool", "tool_call_id": message["tool_calls"][0]["id"], "content": fixture.challenge}])
                else:
                    self.assertIn(fixture.challenge, message["content"])
            self.assertEqual([r["tool_results"] for r in fixture.snapshot()["requests"]], [0, 1, 2])
            self.assertFalse(successful(fixture.snapshot(), {}))

    def test_version_mismatch_has_distinct_outcome(self):
        import sys
        from pathlib import Path
        from verify_acp_model_source import VersionMismatch, locate
        name = Path(sys.executable).name
        entry = {"local_command_candidates": [sys.executable],
                 "installed": {"command": [sys.executable, "--version"]},
                 "distribution": {"release": "0.0.0"},
                 "acp_launch": [name], "underlying": {"kind": "same_distribution"}}
        with self.assertRaises(VersionMismatch):
            locate(entry, {"PATH": ""})

    def test_complete_observations_require_every_lifecycle_stage(self):
        records = [{"model": m, "authenticated": True, "route": "/v1/chat/completions", "tool_results": 2}
                   for m in ("acp-fixture-primary", "acp-fixture-primary", "acp-fixture-primary", "acp-fixture-switched")]
        evidence = {"errors": 0, "requests": records}
        flow = {k: True for k in ("new", "second_turn", "cancel", "restart", "switch", "isolated")}
        self.assertTrue(successful(evidence, flow))
        for key in flow:
            self.assertFalse(successful(evidence, dict(flow, **{key: False})))
        evidence["errors"] = 1
        self.assertFalse(successful(evidence, flow))

    def test_mock_flow_success_output_cannot_replace_model_request(self):
        import argparse
        from pathlib import Path
        import subprocess
        import tempfile
        from unittest.mock import patch
        import verify_acp_model_source as runner

        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"entries": [{"key": "mock-flow-only"}]}))
            output = root / "result.json"
            args = argparse.Namespace(agent="mock-flow-only", mode="session-model", manifest=manifest, output=output)

            def fake_cargo(command, **kwargs):
                self.assertEqual(command[6], runner.TESTS["session-model"])
                path = Path(kwargs["env"]["VW_ACP_ACCEPTANCE_FLOW"])
                path.write_text(json.dumps({k: True for k in ("new", "second_turn", "cancel", "restart", "switch", "isolated", "handshake")}))
                return subprocess.CompletedProcess(command, 0,
                    b'running 1 test\ntest result: ok. 1 passed; 0 failed; 0 ignored; model=acp-fixture-primary', b'')

            with patch.object(runner, "cargo_slot", return_value="/mock-flow/rust-slot-1"), \
                 patch.object(runner, "locate", return_value={"command": "/mock-flow/cli", "args": [], "env": {}}), \
                 patch.object(runner, "isolated_environment", return_value={"MOCK_CONFIG_DIR": str(root)}), \
                 patch.object(runner, "execute_cargo", side_effect=fake_cargo):
                self.assertEqual(runner.run(args), 1)
            report = json.loads(output.read_text())
            self.assertFalse(report["passed"])
            self.assertEqual(report["status"], "handshake-only")
            self.assertEqual(report["requests"]["requests"], [])


    def test_mock_cargo_timeout_reaps_owned_process(self):
        import os
        import signal
        import subprocess
        import sys
        from verify_acp_model_source import execute_cargo
        if os.name != "posix":
            self.skipTest("process group execution explicitly unavailable")
        previous = signal.getsignal(signal.SIGTERM)
        with self.assertRaises(subprocess.TimeoutExpired):
            execute_cargo([sys.executable, "-c", "import time; time.sleep(30)"], env=os.environ.copy(), timeout=0.05)
        self.assertEqual(signal.getsignal(signal.SIGTERM), previous)



class GeminiContract(unittest.TestCase):
    def test_gemini_success_requires_exact_observed_model_path(self):
        flow = {name: True for name in ("new", "second_turn", "cancel", "restart", "switch", "isolated")}
        flow["mode"] = "cli-local"
        record = {"model":"acp-fixture-primary", "authenticated":True,"route":"/v1beta/models/acp-fixture-primary:streamGenerateContent","tool_results":2}
        evidence = {"errors":0,"requests":[dict(record) for _ in range(4)]}
        self.assertTrue(successful(evidence, flow))
        evidence["requests"][0]["route"] = "/v1beta/models/forbidden-native-model:streamGenerateContent"
        self.assertFalse(successful(evidence, flow))
        evidence["requests"][0]["route"] = "/v1beta/models/acp-fixture-primary:countTokens"
        self.assertFalse(successful(evidence, flow))

    def test_chat_gemini_read_uses_declared_absolute_path(self):
        with ModelFixture() as fixture:
            fixture.read_path = "/owned-gemini/challenge.txt"
            body = {"model":fixture.models[0], "messages":[{"role":"user","content":"read twice"}], "tools":[{"type":"function","function":{"name":"read_file","parameters":{"properties":{"absolute_path":{"type":"string"}}}}}]}
            request = urllib.request.Request(fixture.url + "/v1/chat/completions", json.dumps(body).encode(), {"Authorization":"Bearer "+fixture.key,"Content-Type":"application/json"})
            with urllib.request.urlopen(request) as response:
                call = json.load(response)["choices"][0]["message"]["tool_calls"][0]
            self.assertEqual(json.loads(call["function"]["arguments"]), {"absolute_path":fixture.read_path})
            self.assertEqual(fixture.snapshot()["requests"][0]["tool_results"], 0)

    def test_gemini_stream_records_auth_model_and_actual_tool_results(self):
        with ModelFixture() as fixture:
            contents = [{"role": "user", "parts": [{"text": "read twice"}]}]
            for turn in range(3):
                body = {"contents": contents, "tools": [{"functionDeclarations": [{"name": "read_file", "parameters": {"properties": {"absolute_path": {"type": "string"}}}}]}]}
                request = urllib.request.Request(fixture.url + "/v1beta/models/" + fixture.models[0] + ":streamGenerateContent?alt=sse", json.dumps(body).encode(), {"x-goog-api-key": fixture.key, "Content-Type": "application/json"})
                with urllib.request.urlopen(request) as response:
                    result = json.loads(response.read().decode().removeprefix("data: ").strip())
                parts = result["candidates"][0]["content"]["parts"]
                if turn < 2:
                    call = parts[0]["functionCall"]
                    self.assertEqual(call["args"], {"absolute_path": fixture.read_path})
                    contents.extend([{"role": "model", "parts": parts}, {"role": "user", "parts": [{"functionResponse": {"name": call["name"], "response": {"output": fixture.challenge}}}]}])
                else:
                    self.assertIn(fixture.challenge, parts[0]["text"])
            evidence = fixture.snapshot()
            self.assertEqual([r["tool_results"] for r in evidence["requests"]], [0, 1, 2])
            self.assertEqual(evidence["errors"], 0)
            self.assertNotIn(fixture.key, json.dumps(evidence))

    def test_gemini_rejects_bad_auth_and_model(self):
        for model, key in [("acp-fixture-primary", "wrong"), ("forbidden-native-model", None)]:
            with ModelFixture() as fixture:
                request = urllib.request.Request(fixture.url + "/v1beta/models/" + model + ":generateContent", b'{"contents":[]}', {"x-goog-api-key": key or fixture.key, "Content-Type": "application/json"})
                with self.assertRaises(urllib.error.HTTPError):
                    urllib.request.urlopen(request)
                self.assertEqual(fixture.snapshot()["errors"], 1)
                self.assertEqual(fixture.snapshot()["requests"], [])


if __name__ == "__main__":
    unittest.main()
