#!/usr/bin/env python3
"""Loopback model upstream. Never an ACP agent and never a supplier-success proof."""
import copy
import json
import secrets
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


class ModelFixture:
    """Own observations; credentials/bodies are deliberately absent from snapshots."""

    def __init__(self):
        self.key = secrets.token_hex(24)
        self.control = secrets.token_hex(24)
        self.lock = threading.Lock()
        self.records = []
        self.errors = 0
        self.challenge = secrets.token_hex(16)
        self.read_path = "challenge.txt"
        self.models = ("acp-fixture-primary", "acp-fixture-switched")
        fixture = self

        class Handler(BaseHTTPRequestHandler):
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
                if self.path in ("/v1/models", "/v1/chat/completions/models") and self.headers.get("Authorization") == "Bearer " + fixture.key:
                    return self.reply(200, {"object": "list", "data": [
                        {"id": model, "object": "model"} for model in fixture.models
                    ]})
                if self.path != "/_evidence" or self.headers.get("Authorization") != "Bearer " + fixture.control:
                    with fixture.lock:
                        fixture.errors += 1
                    return self.reply(404, {})
                self.reply(200, fixture.snapshot())

            def do_POST(self):
                try:
                    length = int(self.headers.get("Content-Length", "0"))
                    if not 0 < length <= 2_000_000:
                        raise ValueError("body length")
                    body = json.loads(self.rfile.read(length))
                    model = body.get("model")
                    auth = self.headers.get("Authorization") == "Bearer " + fixture.key
                    if self.path != "/v1/chat/completions" or not auth or model not in fixture.models:
                        raise ValueError("unexpected route, authentication or model")
                    messages = body.get("messages", [])
                    if not messages or not any(m.get("role") == "user" for m in messages):
                        raise ValueError("missing prompt")
                    tool_results = [m for m in messages if m.get("role") == "tool"
                                    and fixture.challenge in json.dumps(m.get("content", ""))]
                    tools = body.get("tools", [])
                    with fixture.lock:
                        fixture.records.append({"model": model, "authenticated": True,
                                                "route": self.path, "tool_results": len(tool_results)})
                    if "CANCEL_PROBE" in json.dumps(next((m for m in reversed(messages) if m.get("role") == "user"), {})):
                        time.sleep(5)
                    # Use the real agent's declared read tool; an actual file must be read.
                    read_tool = next((t.get("function", {}) for t in tools
                                      if t.get("function", {}).get("name", "").lower() in
                                      ("read", "read_file", "readfile", "filesystem-read")), None)
                    if len(tool_results) < 2 and read_tool:
                        props = read_tool.get("parameters", {}).get("properties", {})
                        name = next((k for k in ("file_path", "path", "filePath") if k in props), None)
                        if name is None:
                            raise ValueError("unsupported read tool schema")
                        call = {"id": "fixture-read-" + str(len(tool_results)), "type": "function",
                                "function": {"name": read_tool["name"], "arguments": json.dumps({name: fixture.read_path})}}
                        message = {"role": "assistant", "content": None, "tool_calls": [call]}
                        finish = "tool_calls"
                    else:
                        message = {"role": "assistant", "content": "FIXTURE_MODEL_REPLY " + fixture.challenge}
                        finish = "stop"
                    if body.get("stream"):
                        delta = dict(message)
                        if "tool_calls" in delta:
                            delta["tool_calls"] = [dict(delta["tool_calls"][0], index=0)]
                        chunks = [{"id": "fixture", "object": "chat.completion.chunk", "model": model,
                                   "choices": [{"index": 0, "delta": delta, "finish_reason": None}]},
                                  {"id": "fixture", "object": "chat.completion.chunk", "model": model,
                                   "choices": [{"index": 0, "delta": {}, "finish_reason": finish}],
                                   "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}}]
                        data = ("".join("data: " + json.dumps(c) + "\n\n" for c in chunks) + "data: [DONE]\n\n").encode()
                        self.send_response(200)
                        self.send_header("Content-Type", "text/event-stream")
                        self.send_header("Content-Length", str(len(data)))
                        self.end_headers()
                        self.wfile.write(data)
                    else:
                        self.reply(200, {"id": "fixture", "object": "chat.completion", "model": model,
                                         "choices": [{"index": 0, "message": message, "finish_reason": finish}],
                                         "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}})
                except (BrokenPipeError, ConnectionResetError):
                    pass
                except (ValueError, TypeError, KeyError, json.JSONDecodeError):
                    with fixture.lock:
                        fixture.errors += 1
                    self.reply(400, {"error": {"message": "offline fixture contract rejected request"}})

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)

    @property
    def url(self):
        return "http://127.0.0.1:" + str(self.server.server_port)

    def snapshot(self):
        with self.lock:
            return {"requests": copy.deepcopy(self.records), "errors": self.errors}

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
            and all(r.get("authenticated") is True and r.get("route") == "/v1/chat/completions" for r in records)
            and any(r.get("tool_results", 0) >= 2 for r in records)
            and all(flow.get(k) is True for k in ("new", "second_turn", "cancel", "restart", "switch", "isolated")))
