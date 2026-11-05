#!/usr/bin/env python3
"""Owned model HTTP only. Skill discovery and tools must run in the real CLI."""
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import secrets
import re
import shlex
import time
import threading

LIFECYCLE = ("new", "add", "remove", "v2", "isolated", "cancel", "idle_update", "restart", "permission_denied")


class SkillMountFixture:
    def __init__(self, root):
        self.root = Path(root)
        self.name = "mount-" + secrets.token_hex(6)
        self.body_marker = "SKILL_BODY_" + secrets.token_hex(16)
        self.secret = secrets.token_hex(24)
        self.key = secrets.token_hex(24)
        self.models = ("acp-fixture-primary",)
        self.directory = self.root / "skills" / self.name
        (self.directory / "references").mkdir(parents=True)
        (self.directory / "scripts").mkdir()
        self.skill = self.directory / "SKILL.md"
        self.attachment = self.directory / "references" / "value.txt"
        self.attachment.write_text(self.secret)
        self.script = self.directory / "scripts" / "read.sh"
        self.script.write_text('#!/bin/sh\nset -eu\ncat "$(dirname "$0")/../references/value.txt"\n')
        self.script.chmod(0o700)
        self.skill.write_text(f"---\nname: {self.name}\ndescription: Random attachment acceptance\n---\n"
                              f"{self.body_marker}\nRead references/value.txt with the file tool, then execute scripts/read.sh.\n")
        self.prompt = f"Use the discovered skill {self.name}. Read its attachment and execute its script."
        self.calls = {}
        self.call_events = {}
        self.discovery_observations = []
        self.results = set()
        self.records = []
        self.errors = 0
        self.discovery = 0
        self.first_prompt_clean = None
        self.lock = threading.RLock()
        self.server = None
        self.mode = "present"
        self.total_calls = 0
        self.total_results = 0
        self.completed_kinds = set()
        self.phase_first = True
        self.discovered_dir = None

    def snapshot(self):
        with self.lock:
            return {"model_requests": len(self.records), "tool_requests": self.total_calls,
                    "tool_results": self.total_results, "discovery_events": self.discovery,
                    "first_prompt_clean": self.first_prompt_clean is True, "errors": self.errors,
                    "tool_events": [dict(event) for event in self.call_events.values()],
                    "discovery_observations": list(self.discovery_observations),
                    "skill_body_read": "skill" in self.completed_kinds, "attachment_read": "read" in self.completed_kinds, "script_executed": "execute" in self.completed_kinds}

    def answer(self, messages):
        """No secret can enter a reply until correlated actual tool results contain it."""
        for message in messages:
            if message.get("role") != "tool":
                continue
            kind = self.calls.get(message.get("tool_call_id"))
            event = self.call_events.get(message.get("tool_call_id"))
            if not kind or event is None:
                continue
            if not event["result_received"]:
                self.total_results += 1
                event["result_received"] = True
            matched = ((kind == "skill" and self.skill.read_text().split("---\n")[-1].strip() in str(message.get("content", "")))
                       or (kind != "skill" and self.secret in str(message.get("content", ""))))
            event["matched"] = matched
            if matched:
                self.results.add(kind)
                self.completed_kinds.add(kind)
        if not {"skill", "read", "execute"}.issubset(self.results):
            raise ValueError("attachment read and script execution are required")
        return "SKILL_MOUNT_REPLY " + self.secret

    def response(self, body):
        with self.lock:
            messages = body["messages"]
            if not isinstance(messages, list):
                raise ValueError("messages must be an array")
            first_request = not self.records
            self.records.append(True)
            if self.phase_first:
                text = json.dumps(messages)
                if first_request:
                    self.first_prompt_clean = self.secret not in text and self.body_marker not in text
                # Old assistant/tool history is not the current discovery catalog.
                metadata = "\n".join(str(m.get("content", "")) for m in messages if m.get("role") in ("system", "developer", "user"))
                candidates = re.findall(r"(/[^\s<>\"'`\\]*" + re.escape("/" + self.name + "/SKILL.md") + r")", metadata)
                self.discovered_dir = next((Path(path).parent for path in candidates
                    if Path(path) != self.skill and Path(path).is_file()
                    and Path(path).read_bytes() == self.skill.read_bytes()), None)
                discovered = self.discovered_dir is not None
                self.discovery += int(discovered)
                self.discovery_observations.append({"phase": self.mode, "mounted_view": discovered})
                self.phase_first = False
                if self.mode == "added" and self.name + "-added" not in metadata:
                    raise ValueError("added skill was not discovered")
                if self.mode == "absent" and discovered:
                    raise ValueError("removed or other-session skill leaked")
            if self.mode == "absent":
                return {"role": "assistant", "content": "SKILL_ABSENT"}, "stop"
            if not self.discovered_dir:
                raise ValueError("native catalog has no accessible mounted view of the skill")
            try:
                return {"role": "assistant", "content": self.answer(messages)}, "stop"
            except ValueError:
                pass
            kind = "skill" if "skill" not in self.results else ("read" if "read" not in self.results else "execute")
            for raw in body.get("tools", []):
                tool = raw.get("function", raw)
                props = tool.get("parameters", {}).get("properties", {})
                # Use only concrete tool schemas advertised by the actual CLI.
                if kind in ("skill", "read"):
                    field = next((p for p in ("file_path", "path", "absolute_path") if p in props), None)
                    path = self.discovered_dir / ("SKILL.md" if kind == "skill" else "references/value.txt")
                    if field and "read" in tool.get("name", "").lower():
                        arguments = {field: str(path)}
                    else:
                        field = next((p for p in ("command", "cmd") if p in props), None)
                        if not field or not any(p in tool.get("name", "").lower() for p in ("shell", "exec", "bash", "terminal")):
                            continue
                        arguments = {field: "cat " + shlex.quote(str(path))}
                else:
                    field = next((p for p in ("command", "cmd") if p in props), None)
                    if not field or not any(p in tool.get("name", "").lower() for p in ("shell", "exec", "bash", "terminal")):
                        continue
                    arguments = {field: "/bin/sh " + shlex.quote(str(self.discovered_dir / "scripts/read.sh"))}
                required = tool.get("parameters", {}).get("required", [])
                if any(p not in arguments for p in required):
                    continue
                call_id = "mount_" + secrets.token_hex(12)
                self.calls[call_id] = kind
                self.call_events[call_id] = {"phase": self.mode, "kind": kind, "result_received": False, "matched": False}
                self.total_calls += 1
                return {"role": "assistant", "content": None, "tool_calls": [
                    {"id": call_id, "type": "function", "function": {
                        "name": tool["name"], "arguments": json.dumps(arguments)}}]}, "tool_calls"
            raise ValueError("fixed CLI has no reviewed compatible tool schema")

    def __enter__(self):
        fixture = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                if self.path != "/_evidence" or self.headers.get("Authorization") != "Bearer " + fixture.key:
                    self.send_error(403)
                    return
                data = json.dumps(fixture.snapshot()).encode()
                self.send_response(200)
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                try:
                    self.wfile.write(data)
                except (BrokenPipeError, ConnectionResetError):
                    pass

            def do_POST(self):
                if self.path == "/_begin" and self.headers.get("Authorization") == "Bearer " + fixture.key:
                    body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
                    if body.get("mode") not in ("present", "added", "absent", "cancel"):
                        self.send_error(400)
                        return
                    with fixture.lock:
                        fixture.mode = body["mode"]
                        fixture.results.clear()
                        fixture.calls.clear()
                        fixture.phase_first = True
                        fixture.secret = secrets.token_hex(24)
                        fixture.attachment.write_text(fixture.secret)
                    self.send_response(204)
                    self.end_headers()
                    return
                try:
                    if self.path != "/v1/chat/completions" or self.headers.get("Authorization") != "Bearer " + fixture.key:
                        raise ValueError("route or authentication")
                    size = int(self.headers.get("Content-Length", "0"))
                    if not 0 < size <= 8 * 1024 * 1024:
                        raise ValueError("request size")
                    body = json.loads(self.rfile.read(size))
                    if body.get("model") not in fixture.models:
                        raise ValueError("unexpected model")
                    message, finish = fixture.response(body)
                    if fixture.mode == "cancel":
                        time.sleep(2)
                    value = {"id": "owned-skill-fixture", "object": "chat.completion", "model": fixture.models[0],
                             "choices": [{"index": 0, "message": message, "finish_reason": finish}],
                             "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}}
                    if body.get("stream"):
                        delta = dict(message)
                        if "tool_calls" in delta:
                            delta["tool_calls"] = [dict(delta["tool_calls"][0], index=0)]
                        value["object"] = "chat.completion.chunk"
                        value["choices"] = [{"index": 0, "delta": delta, "finish_reason": finish}]
                        data = ("data: " + json.dumps(value) + "\n\ndata: [DONE]\n\n").encode()
                        content_type = "text/event-stream"
                    else:
                        data = json.dumps(value).encode()
                        content_type = "application/json"
                    self.send_response(200)
                except (ValueError, KeyError, TypeError, AttributeError):
                    with fixture.lock:
                        fixture.errors += 1
                    self.send_response(422)
                    data, content_type = b'{"error":"skill fixture rejected request"}', "application/json"
                self.send_header("Content-Type", content_type)
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                try:
                    self.wfile.write(data)
                except (BrokenPipeError, ConnectionResetError):
                    pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.url = "http://127.0.0.1:" + str(self.server.server_port)
        return self

    def __exit__(self, *_args):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()
