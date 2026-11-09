#!/usr/bin/env python3
"""Loopback model upstream. Never an ACP agent and never a supplier-success proof."""
import copy
import json
import secrets
import threading
import time
import urllib.parse
from urllib.parse import urlsplit
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from acp_model_source_kiro_fixture import KiroFixtureMixin


class ModelFixture:
    """Own observations; credentials/bodies are deliberately absent from snapshots."""

    def __init__(self):
        self.key = secrets.token_hex(24)
        self.control = secrets.token_hex(24)
        self.lock = threading.Lock()
        self.records = []
        self.errors = 0
        self.error_kinds = {}
        self.challenge = secrets.token_hex(16)
        self.read_path = "challenge.txt"
        self.second_read_path = None
        self.probe_auxiliary = False
        self.pi_compaction = False
        self.kiro_controls = False
        self.auxiliary_profile = "general-purpose"
        self.auxiliary_model = "haiku"
        self.delay_next_request = False
        self.chat_effort_probe = False
        self.models = ("acp-fixture-primary", "acp-fixture-switched")
        fixture = self

        class Handler(KiroFixtureMixin, BaseHTTPRequestHandler):
            model_fixture = fixture
            def log_message(self, *_args):
                pass

            def reply(self, status, value):
                data = json.dumps(value).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            def do_GET(self):
                if self.path in ("/v1/models", "/v1/chat/completions/models", "/v1/messages/models") and self.headers.get("Authorization") == "Bearer " + fixture.key:
                    return self.reply(200, {"object": "list", "data": [
                        {"id": model, "object": "model"} for model in fixture.models
                    ]})
                if self.path != "/_evidence" or self.headers.get("Authorization") != "Bearer " + fixture.control:
                    category = "get_catalog" if urlsplit(self.path).path in ("/v1/models", "/v1/chat/completions/models", "/v1/messages/models") else "get_other"
                    with fixture.lock:
                        fixture.errors += 1
                        fixture.error_kinds[category] = fixture.error_kinds.get(category, 0) + 1
                    return self.reply(404, {})
                self.reply(200, fixture.snapshot())

            def auxiliary_call(self, body, messages):
                if not fixture.probe_auxiliary or "AUXILIARY_PROBE" not in json.dumps([m.get("content") for m in messages if m.get("role") == "user"]):
                    return None
                if fixture.auxiliary_profile == "openclaw" and any("AUXILIARY_CHILD" in json.dumps(m.get("content")) for m in messages if m.get("role") == "user"):
                    return None
                # The parent delegates once. The real child receives a different prompt
                # and must execute its own file tools before producing the challenge.
                if any("AUXILIARY_CHILD" in json.dumps(m) for m in messages if m.get("role") == "assistant"):
                    return None
                if fixture.auxiliary_profile == "openclaw" and any(
                        tool.get("function", {}).get("name") == "dispatch_action" for tool in body.get("tools", [])):
                    # Fixed 2026.10.10 discovers native tools lazily. The action id was
                    # confirmed through the genuine tool_search response, not an alias.
                    return {"name": "dispatch_action", "input": {"id": "openclaw:core:sessions_spawn", "args": {
                        "task": "AUXILIARY_CHILD: Read " + fixture.read_path + " and " + (fixture.second_read_path or fixture.read_path) + " using your read tool, then return their contents.",
                        "runtime": "subagent", "mode": "run", "cleanup": "delete", "expectsCompletionMessage": False}}}
                for raw in body.get("tools", []):
                    tool = raw.get("function", raw)
                    if tool.get("name", "").lower() not in ("agent", "task", "invoke_sub_agent"):
                        continue
                    properties = tool.get("parameters", tool.get("input_schema", {})).get("properties", {})
                    inputs = {"description": "Read offline challenge", "subagent_type": fixture.auxiliary_profile,
                              "prompt": "AUXILIARY_CHILD: Read " + fixture.read_path + " and " + (fixture.second_read_path or fixture.read_path) + " using your Read tool, then return their contents."}
                    if tool.get("name") == "invoke_sub_agent":
                        inputs = {"name": "general-task-execution", "prompt": inputs["prompt"],
                                  "explanation": "Read the offline challenge with the native child", "contextFiles": []}
                    if "model" in properties and fixture.auxiliary_model is not None:
                        inputs["model"] = fixture.auxiliary_model
                    return {"name": tool["name"], "input": inputs}
                raise ValueError("auxiliary tool is unavailable")

            def gemini_reply(self, body):
                path = urllib.parse.urlsplit(self.path).path
                resource = urllib.parse.unquote(path.removeprefix("/v1beta/models/"))
                model, separator, action = resource.rpartition(":")
                if not separator or action not in ("generateContent", "streamGenerateContent", "countTokens"):
                    raise ValueError("gemini route")
                if self.headers.get("x-goog-api-key") != fixture.key:
                    raise ValueError("gemini auth")
                if model not in fixture.models:
                    raise ValueError("gemini model")
                contents = body.get("contents", [])
                if not contents:
                    raise ValueError("gemini prompt")
                if action == "countTokens":
                    return self.reply(200, {"totalTokens": 1})
                results = [part["functionResponse"] for content in contents for part in content.get("parts", [])
                           if "functionResponse" in part and fixture.challenge in json.dumps(part["functionResponse"].get("response", {}))]
                with fixture.lock:
                    fixture.records.append({"model": model, "authenticated": True, "route": path,
                                            "tool_results": len(results), "auxiliary": False})
                if "CANCEL_PROBE" in json.dumps(next((c for c in reversed(contents) if c.get("role") == "user"), {})):
                    time.sleep(5)
                tools = [tool for group in body.get("tools", []) for tool in group.get("functionDeclarations", [])]
                read = next((tool for tool in tools if tool.get("name", "").lower() in ("read_file", "readfile", "read")), None)
                if len(results) < 2 and read:
                    properties = read.get("parameters", read.get("parametersJsonSchema", {})).get("properties", {})
                    argument = next((key for key in ("absolute_path", "file_path", "path", "filePath") if key in properties), None)
                    if not argument:
                        raise ValueError("gemini read schema")
                    parts = [{"functionCall": {"name": read["name"], "args": {argument: fixture.read_path}}}]
                else:
                    parts = [{"text": "FIXTURE_MODEL_REPLY " + fixture.challenge}]
                response = {"candidates": [{"content": {"role": "model", "parts": parts}, "finishReason": "STOP", "index": 0}],
                            "usageMetadata": {"promptTokenCount": 1, "candidatesTokenCount": 1, "totalTokenCount": 2}}
                if action != "streamGenerateContent":
                    return self.reply(200, response)
                data = ("data: " + json.dumps(response) + "\n\n").encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                try:
                    self.wfile.write(data)
                except (BrokenPipeError, ConnectionResetError):
                    pass

            def do_POST(self):
                try:
                    length = int(self.headers.get("Content-Length", "0"))
                    if not 0 < length <= 2_000_000:
                        raise ValueError("body length")
                    body = json.loads(self.rfile.read(length))
                    if self.path == "/_cancel-next" and self.headers.get("Authorization") == "Bearer " + fixture.control:
                        with fixture.lock:
                            fixture.delay_next_request = True
                        return self.reply(200, {"armed": True})
                    if self.path.startswith("/v1beta/models/"):
                        return self.gemini_reply(body)
                    route = urlsplit(self.path).path
                    if route == "/" and fixture.kiro_controls:
                        return self.kiro_control(body)
                    if route == "/v1/responses":
                        return self.responses_reply(body)
                    if route in ("/v1/messages", "/v1/messages/count_tokens"):
                        return self.messages_reply(body, route)
                    model = body.get("model")
                    auth = self.headers.get("Authorization") == "Bearer " + fixture.key
                    if self.path != "/v1/chat/completions":
                        raise ValueError("route")
                    if not auth:
                        raise ValueError("auth")
                    if model not in fixture.models:
                        raise ValueError("model")
                    messages = body.get("messages", [])
                    if not messages or not any(m.get("role") == "user" for m in messages):
                        raise ValueError("missing prompt")
                    tool_results = [m for m in messages if m.get("role") == "tool"
                                    and fixture.challenge in json.dumps(m.get("content", ""))]
                    tools = body.get("tools", [])
                    # Fixed Pi 0.73.1 uses this native summarizer for both history
                    # and split-turn compaction; the latter omits custom focus text.
                    pi_summary = (fixture.pi_compaction and not tools
                        and "You are a context summarization assistant." in json.dumps([
                            m.get("content") for m in messages if m.get("role") == "system"])
                        and any("<conversation>" in json.dumps(m.get("content"))
                            and "</conversation>" in json.dumps(m.get("content"))
                            for m in messages if m.get("role") == "user"))
                    with fixture.lock:
                        delay_next_request = fixture.delay_next_request
                        fixture.delay_next_request = False
                        record = {"model": model, "authenticated": True,
                                  "route": self.path, "tool_results": len(tool_results),
                                  "auxiliary": pi_summary or any("AUXILIARY_CHILD" in json.dumps(m.get("content")) for m in messages if m.get("role") == "user")
                                  and (fixture.auxiliary_profile != "openclaw" or not any("AUXILIARY_PROBE" in json.dumps(m.get("content")) for m in messages if m.get("role") == "user"))}
                        if fixture.chat_effort_probe:
                            user = next(m for m in reversed(messages) if m.get("role") == "user")
                            content = json.dumps(user.get("content"))
                            record["turn"] = next((tag for tag in ("DEFAULT", "A", "B")
                                                   if tools and json.dumps("ACP_EFFORT_" + tag) in content), "auxiliary")
                            effort = body.get("reasoning_effort")
                            record["reasoning_effort"] = (effort if effort in ("low", "high")
                                                          else "other" if "reasoning_effort" in body else None)
                        fixture.records.append(record)
                    if delay_next_request or "CANCEL_PROBE" in json.dumps(next((m for m in reversed(messages) if m.get("role") == "user"), {})):
                        time.sleep(5)
                    # Use the real agent's declared read tool; an actual file must be read.
                    read_tool = next((t.get("function", {}) for t in tools
                                      if t.get("function", {}).get("name", "").lower() in
                                      ("read", "read_file", "readfile", "filesystem-read")), None)
                    auxiliary = self.auxiliary_call(body, messages)
                    if fixture.auxiliary_profile == "openclaw" and any("AUXILIARY_CHILD" in json.dumps(m) for m in messages if m.get("role") == "assistant"):
                        deadline = time.monotonic() + 10
                        while time.monotonic() < deadline:
                            with fixture.lock:
                                child_done = any(r.get("auxiliary") and r.get("tool_results", 0) >= 2 for r in fixture.records)
                            if child_done:
                                break
                            time.sleep(0.05)
                    delegated = any("AUXILIARY_CHILD" in json.dumps(m) for m in messages if m.get("role") == "assistant")
                    if auxiliary:
                        call = {"id": "fixture-agent-" + str(len(fixture.records)), "type": "function",
                                "function": {"name": auxiliary["name"], "arguments": json.dumps(auxiliary["input"])}}
                        message = {"role": "assistant", "content": None, "tool_calls": [call]}
                        finish = "tool_calls"
                    elif not fixture.chat_effort_probe and len(tool_results) < 2 and read_tool and not (delegated and tool_results):
                        props = read_tool.get("parameters", {}).get("properties", {})
                        name = next((k for k in ("absolute_path", "file_path", "path", "filePath") if k in props), None)
                        if name is None:
                            raise ValueError("unsupported read tool schema")
                        call = {"id": "fixture-read-" + str(len(fixture.records)), "type": "function",
                                "function": {"name": read_tool["name"], "arguments": json.dumps({name: (fixture.second_read_path or fixture.read_path) if tool_results else fixture.read_path})}}
                        message = {"role": "assistant", "content": None, "tool_calls": [call]}
                        finish = "tool_calls"
                    else:
                        message = {"role": "assistant", "content": "FIXTURE_MODEL_REPLY " + fixture.challenge}
                        finish = "stop"
                    response_id = "fixture-" + secrets.token_hex(12)
                    if body.get("stream"):
                        delta = dict(message)
                        if "tool_calls" in delta:
                            delta["tool_calls"] = [dict(delta["tool_calls"][0], index=0)]
                        chunks = [{"id": response_id, "object": "chat.completion.chunk", "model": model,
                                   "choices": [{"index": 0, "delta": delta, "finish_reason": None}]},
                                  {"id": response_id, "object": "chat.completion.chunk", "model": model,
                                   "choices": [{"index": 0, "delta": {}, "finish_reason": finish}],
                                   "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}}]
                        data = ("".join("data: " + json.dumps(c) + "\n\n" for c in chunks) + "data: [DONE]\n\n").encode()
                        self.send_response(200)
                        self.send_header("Content-Type", "text/event-stream")
                        self.send_header("Content-Length", str(len(data)))
                        self.end_headers()
                        self.wfile.write(data)
                    else:
                        self.reply(200, {"id": response_id, "object": "chat.completion", "model": model,
                                         "choices": [{"index": 0, "message": message, "finish_reason": finish}],
                                         "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}})
                except (BrokenPipeError, ConnectionResetError):
                    pass
                except (ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
                    # Only closed diagnostic categories: no paths, auth or request bodies.
                    category = str(error) if str(error) in ("route", "auth", "model") else "contract"
                    with fixture.lock:
                        fixture.errors += 1
                        fixture.error_kinds[category] = fixture.error_kinds.get(category, 0) + 1
                    self.reply(400, {"error": {"message": "offline fixture contract rejected request"}})

            def messages_reply(self, body, route):
                authenticated = (self.headers.get("Authorization") == "Bearer " + fixture.key
                                 or self.headers.get("x-api-key") == fixture.key)
                if not authenticated:
                    raise ValueError("auth")
                if body.get("model") not in fixture.models:
                    raise ValueError("model")
                if route.endswith("/count_tokens"):
                    return self.reply(200, {"input_tokens": 1})
                messages = body.get("messages", [])
                if not any(message.get("role") == "user" for message in messages):
                    raise ValueError("missing Messages prompt")
                results = [block for message in messages if message.get("role") == "user"
                           for block in (message.get("content") if isinstance(message.get("content"), list) else [])
                           if block.get("type") == "tool_result" and fixture.challenge in json.dumps(block.get("content"))]
                with fixture.lock:
                    fixture.records.append({"model": body["model"], "authenticated": True,
                                            "route": route, "tool_results": len(results),
                                            "auxiliary": any("AUXILIARY_CHILD" in json.dumps(m.get("content")) for m in messages if m.get("role") == "user")})
                if "CANCEL_PROBE" in json.dumps(next((m for m in reversed(messages) if m.get("role") == "user"), {})):
                    time.sleep(5)
                tool = next((tool for tool in body.get("tools", [])
                             if tool.get("name", "").lower() in ("read", "read_file", "readfile", "filesystem-read")), None)
                auxiliary = self.auxiliary_call(body, messages)
                delegated = any("AUXILIARY_CHILD" in json.dumps(m) for m in messages if m.get("role") == "assistant")
                if auxiliary:
                    block = {"type": "tool_use", "id": "fixture-agent-" + str(len(fixture.records)), **auxiliary}
                    reason = "tool_use"
                elif len(results) < 2 and tool and not (delegated and results):
                    properties = tool.get("input_schema", {}).get("properties", {})
                    name = next((name for name in ("file_path", "path", "filePath") if name in properties), None)
                    if name is None:
                        raise ValueError("unsupported Messages read tool schema")
                    block = {"type": "tool_use", "id": "fixture-read-" + str(len(fixture.records)),
                             "name": tool["name"], "input": {name: (fixture.second_read_path or fixture.read_path) if results else fixture.read_path}}
                    reason = "tool_use"
                else:
                    block = {"type": "text", "text": "FIXTURE_MODEL_REPLY " + fixture.challenge}
                    reason = "end_turn"
                message = {"id": "msg_fixture_" + str(len(fixture.records)), "type": "message", "role": "assistant", "model": body["model"],
                           "content": [block], "stop_reason": reason, "stop_sequence": None,
                           "usage": {"input_tokens": 1, "output_tokens": 1}}
                if not body.get("stream"):
                    return self.reply(200, message)
                start = dict(message, content=[], stop_reason=None, usage={"input_tokens": 1, "output_tokens": 0})
                empty_block = dict(block, **({"input": {}} if block["type"] == "tool_use" else {"text": ""}))
                delta = ({"type": "input_json_delta", "partial_json": json.dumps(block["input"])}
                         if block["type"] == "tool_use" else {"type": "text_delta", "text": block["text"]})
                events = [
                    {"type": "message_start", "message": start},
                    {"type": "content_block_start", "index": 0, "content_block": empty_block},
                    {"type": "content_block_delta", "index": 0, "delta": delta},
                    {"type": "content_block_stop", "index": 0},
                    {"type": "message_delta", "delta": {"stop_reason": reason, "stop_sequence": None}, "usage": {"output_tokens": 1}},
                    {"type": "message_stop"},
                ]
                data = "".join("event: " + event["type"] + "\ndata: " + json.dumps(event) + "\n\n" for event in events).encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)

    @property
    def url(self):
        return "http://127.0.0.1:" + str(self.server.server_port)

    def snapshot(self):
        with self.lock:
            result = {"requests": copy.deepcopy(self.records), "errors": self.errors}
            if self.error_kinds:
                result["error_kinds"] = dict(self.error_kinds)
            return result

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *_args):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()


def successful(evidence, flow):
    """Neither output text nor declared model fields are model observations."""
    records = evidence.get("requests", [])
    models = {r.get("model") for r in records}
    return (evidence.get("errors") == 0 and len(records) >= 4
            and models == ({"acp-fixture-primary"} if flow.get("mode") == "cli-local" else {"acp-fixture-primary", "acp-fixture-switched"})
            and all(r.get("authenticated") is True and r.get("route") in ("/v1/chat/completions", "/v1/messages", "/v1/responses",
                "/v1beta/models/" + str(r.get("model")) + ":generateContent",
                "/v1beta/models/" + str(r.get("model")) + ":streamGenerateContent") for r in records)
            and any(r.get("tool_results", 0) >= 2 for r in records)
            and (not flow.get("requires_auxiliary") or any(r.get("auxiliary") is True for r in records))
            and all(flow.get(k) is True for k in ("new", "second_turn", "cancel", "restart", "switch", "isolated")))
