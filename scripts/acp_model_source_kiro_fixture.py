"""Kiro's external control plane and Responses model fixture; never an ACP agent."""
import hashlib
import json
import secrets
import time


class KiroFixtureMixin:
    def kiro_control(self, body):
        fixture = self.model_fixture
        if not fixture.kiro_controls or self.headers.get("Authorization") != "Bearer " + fixture.key:
            raise ValueError("kiro control auth")
        target = self.headers.get("x-amz-target")
        if target == "KiroControlPlaneBearerService.ListAvailableModels":
            return self.reply(200, {"models": [{"modelId": m, "modelName": m, "modelProvider": "fixture"} for m in fixture.models],
                                    "defaultModel": {"modelId": fixture.models[0]}})
        if target == "KiroRuntimeService.GetFeatureConfiguration":
            key = hashlib.sha256(b"kiro-feature-config-key-salt-3e9b1d7acreate_response_api").hexdigest()
            return self.reply(200, {"configuration": {key: "create_response"}})
        if target in ("KiroControlPlaneBearerService.GetProfile", "KiroControlPlaneBearerService.GetEffectiveEnterpriseManagementResourceWithToken"):
            return self.reply(200, {})
        if target == "KiroRuntimeService.InvokeMCP" and body.get("method") == "tools/list":
            return self.reply(200, {"jsonrpc": "2.0", "id": body.get("id"), "result": {"tools": []}})
        raise ValueError("unsupported kiro control operation")

    def responses_reply(self, body):
        fixture = self.model_fixture
        if self.headers.get("Authorization") != "Bearer " + fixture.key or body.get("model") not in fixture.models:
            raise ValueError("responses auth/model")
        inputs = body.get("input", [])
        if not isinstance(inputs, list) or not inputs:
            raise ValueError("responses input")
        messages = []
        for item in inputs:
            if item.get("type") == "function_call_output":
                messages.append({"role": "tool", "content": item.get("output")})
            elif item.get("type") == "function_call":
                messages.append({"role": "assistant", "content": item.get("arguments")})
            elif item.get("role"):
                messages.append(item)
        results = [m for m in messages if m.get("role") == "tool" and fixture.challenge in json.dumps(m.get("content"))]
        auxiliary = any("AUXILIARY_CHILD" in json.dumps(m.get("content")) for m in messages if m.get("role") == "user")
        with fixture.lock:
            fixture.records.append({"model": body["model"], "authenticated": True, "route": "/v1/responses", "tool_results": len(results), "auxiliary": auxiliary})
        if "CANCEL_PROBE" in json.dumps(next((m for m in reversed(messages) if m.get("role") == "user"), {})):
            time.sleep(5)
        tools = body.get("tools", [])
        delegate = self.auxiliary_call(body, messages)
        read = next((t for t in tools if t.get("name", "").lower() in ("read", "read_file", "readfile", "filesystem-read")), None)
        call = None
        if delegate:
            call = delegate
        elif len(results) < 2 and read:
            props = read.get("parameters", {}).get("properties", {})
            key = next((k for k in ("absolute_path", "file_path", "path", "filePath") if k in props), None)
            if key is None:
                raise ValueError("responses read schema")
            call = {"name": read["name"], "input": {key: (fixture.second_read_path or fixture.read_path) if results else fixture.read_path}}
        response_id = "resp_fixture_" + secrets.token_hex(12)
        if call:
            item = {"type": "function_call", "id": "fc_" + secrets.token_hex(8), "call_id": "call_" + secrets.token_hex(8),
                    "name": call["name"], "arguments": json.dumps(call["input"]), "status": "completed"}
            events = [{"type": "response.output_item.added", "output_index": 0, "item": dict(item, arguments="")},
                      {"type": "response.function_call_arguments.delta", "output_index": 0, "item_id": item["id"], "delta": item["arguments"]},
                      {"type": "response.output_item.done", "output_index": 0, "item": item}]
        else:
            item = {"type": "message", "id": "msg_fixture", "role": "assistant", "status": "completed",
                    "content": [{"type": "output_text", "text": "FIXTURE_MODEL_REPLY " + fixture.challenge, "annotations": []}]}
            events = [{"type": "response.output_text.delta", "delta": item["content"][0]["text"]}]
        response = {"id": response_id, "object": "response", "status": "completed", "model": body["model"], "output": [item],
                    "usage": {"input_tokens": 1, "output_tokens": 1, "total_tokens": 2}}
        if not body.get("stream"):
            return self.reply(200, response)
        events.insert(0, {"type": "response.created", "response": dict(response, status="in_progress", output=[])})
        events.append({"type": "response.completed", "response": response})
        data = "".join("data: " + json.dumps(e) + "\n\n" for e in events).encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        try:
            self.wfile.write(data)
        except (BrokenPipeError, ConnectionResetError):
            pass
