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
        self.error_categories = []
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
        self.native_skill = None
        self.native_catalog = {}
        self.native_skill_invocations = 0
        self.added_confirmed = False
        self.gemini_calls = {}

    def snapshot(self):
        with self.lock:
            return {"model_requests": len(self.records), "tool_requests": self.total_calls,
                    "tool_results": self.total_results, "discovery_events": self.discovery,
                    "first_prompt_clean": self.first_prompt_clean is True, "errors": self.errors,
                    "native_catalog": self.native_catalog, "error_categories": list(self.error_categories),
                    "native_skill_invocations": self.native_skill_invocations,
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
            matched = ((kind == "added" and "ADDED_SKILL_BODY" in str(message.get("content", "")))
                       or (kind == "skill" and self.skill.read_text().split("---\n")[-1].strip() in str(message.get("content", "")))
                       or (kind not in ("skill", "added") and self.secret in str(message.get("content", ""))))
            event["matched"] = matched
            if matched:
                self.results.add(kind)
                self.completed_kinds.add(kind)
        if not {"skill", "read", "execute"}.issubset(self.results):
            raise ValueError("attachment read and script execution are required")
        return "SKILL_MOUNT_REPLY " + self.secret

    def native_skill_call(self, body, qualified, kind):
        for raw in body.get("tools", []):
            tool = raw.get("function", raw)
            field = "name" if (getattr(self, "gemini_native", False) or (getattr(self, "goose_native", False) or (getattr(self, "vibe_native", False) or getattr(self, "hermes_native", False)))) else "skill"
            expected = "activate_skill" if getattr(self, "gemini_native", False) else ("skill" if getattr(self, "qwen_native", False) else "Skill")
            if (getattr(self, "goose_native", False) or (getattr(self, "vibe_native", False) or getattr(self, "hermes_native", False))):
                expected = "skill_view" if getattr(self, "hermes_native", False) else "skill" if getattr(self, "vibe_native", False) else "load_skill"
            if tool.get("name") == expected and field in tool.get("parameters", {}).get("properties", {}):
                call_id = "mount_" + secrets.token_hex(12)
                self.calls[call_id] = kind
                self.native_skill_invocations += 1
                self.call_events[call_id] = {"phase":self.mode, "kind":kind, "result_received":False, "matched":False}
                self.total_calls += 1
                return {"role":"assistant", "content":None, "tool_calls":[{"id":call_id, "type":"function", "function":{"name":tool["name"], "arguments":json.dumps({field:qualified})}}]}, "tool_calls"
        raise ValueError("fixed CLI has no reviewed compatible tool schema")

    def response(self, body):
        # Native clients also send tool-free utility requests. These supply no
        # discovery/tool evidence and must not advance the acceptance phase.
        if (getattr(self, "codebuddy_native", False) or (getattr(self, "vibe_native", False) or getattr(self, "hermes_native", False))) and not body.get("tools"):
            return {"role": "assistant", "content": "Ready"}, "stop"
        with self.lock:
            native_mode = any(getattr(self, key, False) for key in ("claude_native", "qwen_native", "gemini_native", "codebuddy_native", "goose_native", "vibe_native", "hermes_native"))
            qualified = self.name if (getattr(self, "codebuddy_native", False) or getattr(self, "qwen_native", False) or getattr(self, "gemini_native", False) or (getattr(self, "goose_native", False) or (getattr(self, "vibe_native", False) or getattr(self, "hermes_native", False)))) else "shiju-session-skills:" + self.name
            messages = body["messages"]
            if getattr(self, "qwen_native", False) or getattr(self, "gemini_native", False) or (getattr(self, "goose_native", False) or (getattr(self, "vibe_native", False) or getattr(self, "hermes_native", False))):
                # Native clients can serialize function responses as JSON objects/arrays.
                # Decode the actual correlated result instead of comparing JSON escapes.
                def result_text(value):
                    if isinstance(value, str):
                        if value.lstrip().startswith(("[", "{")):
                            try:
                                return result_text(json.loads(value))
                            except ValueError:
                                pass
                        return value
                    if isinstance(value, list):
                        return "\n".join(result_text(item) for item in value)
                    if isinstance(value, dict):
                        return "\n".join(result_text(item) for item in value.values())
                    return ""
                messages = [dict(message, content=result_text(message.get("content")))
                            if message.get("role") == "tool" else message for message in messages]
            if not isinstance(messages, list):
                raise ValueError("messages must be an array")
            if body.get("claude_native") or getattr(self, "claude_native", False):
                # SDK 0.2.96 returns a correlated launch result, then a separate user
                # block containing the actual on-disk skill expansion and base path.
                normalized = []
                for index, message in enumerate(messages):
                    kind = self.calls.get(message.get("tool_call_id"))
                    target = "shiju-session-skills:" + self.name + ("-added" if kind == "added" else "")
                    marker = "ADDED_SKILL_BODY" if kind == "added" else self.body_marker
                    if (message.get("role") == "tool" and kind in ("skill", "added")
                            and "Launching skill: " + target in str(message.get("content", ""))):
                        expansion = next((str(later.get("content", "")) for later in messages[index + 1:]
                            if later.get("role") == "user" and marker in str(later.get("content", ""))
                            and "Base directory for this skill:" in str(later.get("content", ""))), "")
                        message = dict(message, content=str(message.get("content", "")) + "\n" + expansion)
                    normalized.append(message)
                messages = normalized
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
                self.native_skill = None
                if body.get("claude_native") or native_mode:
                    catalog = metadata + json.dumps(body.get("tools", []))
                    if getattr(self, "qwen_native", False) or getattr(self, "gemini_native", False):
                        catalog = "\n".join(re.findall(r"<available_skills>(.*?)</available_skills>", metadata, re.S))
                    if (getattr(self, "codebuddy_native", False) or getattr(self, "qwen_native", False) or getattr(self, "gemini_native", False) or (getattr(self, "goose_native", False) or (getattr(self, "vibe_native", False) or getattr(self, "hermes_native", False)))) and getattr(self, "forbidden_skill", "") and self.forbidden_skill in catalog:
                        raise ValueError("unselected native skill leaked")
                    if getattr(self, "goose_native", False):
                        # Only Goose's native system catalog is discovery evidence.
                        catalog = "\n".join(str(m.get("content", "")) for m in messages if m.get("role") == "system")
                        sections = "\n".join(re.findall(r"# Skills\n\nYou have these skills at your disposal\..*?(?=\n# |\Z)", catalog, re.S))
                        names = set(re.findall(r"^- (\S+): ", sections, re.M))
                        expected = set() if self.mode == "absent" else {self.name}
                        if self.mode == "added":
                            expected.add(self.name + "-added")
                        if names != expected:
                            self.native_catalog = {"observed_count": len(names), "expected_count": len(expected),
                                "selected_present": self.name in names,
                                "tool_names": [t.get("function", t).get("name") for t in body.get("tools", [])]}
                            raise ValueError("Goose native catalog does not equal selected skills")
                    if getattr(self, "vibe_native", False):
                        self.discovered_dir = None
                        catalog = "\n".join(str(m.get("content", "")) for m in messages if m.get("role") == "system")
                        names = set(re.findall(r"<skill>\s*<name>([^<]+)</name>", catalog))
                        expected = set() if self.mode == "absent" else {self.name}
                        if self.mode == "added":
                            expected.add(self.name + "-added")
                        if names != expected:
                            self.native_catalog = {"phase": self.mode, "observed_count": len(names), "expected_count": len(expected),
                                "selected_present": self.name in names, "observed_names": sorted(names),
                                "tool_names": [t.get("function", t).get("name") for t in body.get("tools", [])]}
                            raise ValueError("Vibe native catalog does not equal selected skills")
                    if getattr(self, "hermes_native", False):
                        self.discovered_dir = None
                        catalog = "\n".join(str(m.get("content", "")) for m in messages if m.get("role") == "system")
                        sections = "\n".join(re.findall(r"<available_skills>(.*?)</available_skills>", catalog, re.S))
                        names = set(re.findall(r"^    - ([^:\s]+)", sections, re.M))
                        expected = set() if self.mode == "absent" else {self.name}
                        if self.mode == "added":
                            expected.add(self.name + "-added")
                        if names != expected:
                            self.native_catalog = {"phase": self.mode, "observed_count": len(names), "expected_count": len(expected), "selected_present": self.name in names}
                            raise ValueError("Hermes native catalog does not equal selected skills")
                        catalog = sections
                    if getattr(self, "codebuddy_native", False):
                        tool_catalog = "\n".join(t.get("function", t).get("description", "") for t in body.get("tools", []) if t.get("function", t).get("name") == "Skill")
                        sections = "\n".join(re.findall(r"<available_skills>(.*?)</available_skills>", tool_catalog, re.S))
                        names = set(re.findall(r"^- (\S+?):\s", sections, re.M))
                        expected = set() if self.mode == "absent" else {self.name}
                        if self.mode == "added":
                            expected.add(self.name + "-added")
                        if names != expected:
                            self.native_catalog = {"phase": self.mode, "observed_count": len(names), "expected_count": len(expected),
                                "selected_present": self.name in names, "added_present": self.name + "-added" in names, "builtin_loop_present": "loop" in names, "tool_names": [t.get("function",t).get("name") for t in body.get("tools",[])]}
                            raise ValueError("unselected native skill leaked or selected skill missing")
                        # Only the current tool descriptor is an authoritative catalog;
                        # resumed user/system history can contain older skill overviews.
                        catalog = sections
                    self.native_catalog = {"selected_name_in_metadata": self.name in metadata,
                        "selected_name_in_tools": self.name in json.dumps(body.get("tools", [])),
                        "tool_names": [t.get("function", t).get("name") for t in body.get("tools", [])]}
                    if qualified in catalog:
                        self.native_skill = qualified
                        if getattr(self, "gemini_native", False) or (getattr(self, "goose_native", False) or (getattr(self, "vibe_native", False) or getattr(self, "hermes_native", False))):
                            # Require the actual activation result before reading resources.
                            self.discovered_dir = None
                discovered = self.discovered_dir is not None or self.native_skill is not None
                self.discovery += int(discovered and not (native_mode and self.mode == "absent"))
                self.discovery_observations.append({"phase": self.mode, "mounted_view": discovered})
                self.phase_first = False
                if self.mode == "added" and not native_mode and self.name + "-added" not in metadata:
                    raise ValueError("added skill was not discovered")
                if self.mode == "absent" and discovered and not native_mode:
                    raise ValueError("removed or other-session skill leaked")
            if self.mode == "absent":
                if ((getattr(self, "vibe_native", False) or getattr(self, "hermes_native", False)) and not self.native_skill
                        and not any(t.get("function", t).get("name") == ("skill_view" if getattr(self, "hermes_native", False) else "skill") for t in body.get("tools", []))):
                    self.discovery_observations[-1]["native_activation_unregistered"] = True
                    return {"role": "assistant", "content": "SKILL_ABSENT"}, "stop"
                if ((getattr(self, "gemini_native", False) or getattr(self, "codebuddy_native", False)) and not self.native_skill
                        and not any(t.get("function", t).get("name") == ("Skill" if getattr(self, "codebuddy_native", False) else "activate_skill") for t in body.get("tools", []))):
                    self.discovery_observations[-1]["native_activation_unregistered"] = True
                    return {"role": "assistant", "content": "SKILL_ABSENT"}, "stop"
                if native_mode:
                    for message in messages:
                        if message.get("role") == "tool" and self.calls.get(message.get("tool_call_id")) == "missing":
                            content = str(message.get("content", ""))
                            if not (("Unknown skill: " + qualified in content) or ((getattr(self, "codebuddy_native", False) or getattr(self, "qwen_native", False) or getattr(self, "gemini_native", False) or (getattr(self, "goose_native", False) or (getattr(self, "vibe_native", False) or getattr(self, "hermes_native", False)))) and qualified in content and ("not found" in content or "not available to this runtime" in content))):
                                raise ValueError("removed or other-session skill leaked")
                            event = self.call_events[message["tool_call_id"]]
                            if not event["result_received"]:
                                self.total_results += 1
                            event.update(result_received=True, matched=True)
                            self.discovery_observations[-1]["mounted_view"] = False
                            self.discovery_observations[-1]["native_rejection_verified"] = True
                            return {"role":"assistant", "content":"SKILL_ABSENT"}, "stop"
                    return self.native_skill_call(body, qualified, "missing")
                return {"role": "assistant", "content": "SKILL_ABSENT"}, "stop"
            if self.mode == "added" and native_mode and not self.added_confirmed:
                for message in messages:
                    if message.get("role") == "tool" and self.calls.get(message.get("tool_call_id")) == "added":
                        if "ADDED_SKILL_BODY" not in str(message.get("content", "")):
                            raise ValueError("added skill was not discovered")
                        event = self.call_events[message["tool_call_id"]]
                        if not event["result_received"]:
                            self.total_results += 1
                        event.update(result_received=True, matched=True)
                        self.added_confirmed = True
                        self.discovery_observations[-1]["added_native_verified"] = True
                if not self.added_confirmed:
                    return self.native_skill_call(body, qualified + "-added", "added")
            if self.native_skill and not self.discovered_dir:
                # Learn the view path from the actual native Skill result, never the fixture source.
                for message in messages:
                    if message.get("role") != "tool" or self.calls.get(message.get("tool_call_id")) != "skill":
                        continue
                    paths = re.findall(r"(/[^\s<>\"'`\\]*" + re.escape("/" + self.name) + r")", str(message.get("content", "")))
                    self.discovered_dir = next((Path(path) for path in paths
                        if Path(path) != self.directory and (Path(path) / "SKILL.md").is_file()
                        and (Path(path) / "SKILL.md").read_bytes() == self.skill.read_bytes()), None)
                if not self.discovered_dir:
                    if any(kind == "skill" for kind in self.calls.values()):
                        raise ValueError("native Skill result did not identify the mounted view")
                    return self.native_skill_call(body, self.native_skill, "skill")
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
                if getattr(fixture, "hermes_native", False) and self.path in ("/api/v1/models", "/api/tags", "/v1/props", "/props", "/version"):
                    self.send_error(404)
                    return
                if (getattr(fixture, "goose_native", False) or getattr(fixture, "hermes_native", False)) and self.path.startswith("/v1/models"):
                    if self.headers.get("Authorization") != "Bearer " + fixture.key:
                        self.send_error(403)
                        return
                    data = json.dumps({"data": [{"id": fixture.models[0]}]} if self.path == "/v1/models" else
                                      {"id": self.path.rsplit("/", 1)[1], "context_length": 128000}).encode()
                    self.send_response(200)
                    self.send_header("Content-Length", str(len(data)))
                    self.end_headers()
                    self.wfile.write(data)
                    return
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
                        fixture.added_confirmed = False
                        fixture.calls.clear()
                        fixture.phase_first = True
                        fixture.secret = secrets.token_hex(24)
                        fixture.attachment.write_text(fixture.secret)
                    self.send_response(204)
                    self.end_headers()
                    return
                try:
                    route = self.path.split("?", 1)[0]
                    gemini = route.startswith("/v1beta/models/") and getattr(fixture, "gemini_native", False)
                    authenticated = (self.headers.get("Authorization") == "Bearer " + fixture.key
                                     or route == "/v1/messages" and self.headers.get("x-api-key") == fixture.key
                                     or gemini and self.headers.get("x-goog-api-key") == fixture.key)
                    if (not gemini and route not in ("/v1/chat/completions", "/v1/responses", "/v1/messages")) or not authenticated:
                        raise ValueError("route or authentication")
                    size = int(self.headers.get("Content-Length", "0"))
                    if not 0 < size <= 8 * 1024 * 1024:
                        raise ValueError("request size")
                    body = json.loads(self.rfile.read(size))
                    if gemini:
                        from urllib.parse import unquote
                        model, _, action = unquote(route.removeprefix("/v1beta/models/")).rpartition(":")
                        if action not in ("generateContent", "streamGenerateContent", "countTokens"):
                            raise ValueError("route or authentication")
                        body["model"] = model
                    if body.get("model") not in fixture.models:
                        raise ValueError("unexpected model")
                    if gemini and action == "countTokens":
                        data = b'{"totalTokens":1}'
                        self.send_response(200)
                        self.send_header("Content-Type", "application/json")
                        self.send_header("Content-Length", str(len(data)))
                        self.end_headers()
                        self.wfile.write(data)
                        return
                    if gemini:
                        from acp_skill_mount_gemini import generate_message
                        message, finish = generate_message(fixture, body)
                    elif route == "/v1/messages":
                        from acp_skill_mount_claude import messages_message
                        message, finish = messages_message(fixture, body)
                    elif route == "/v1/responses":
                        from acp_skill_mount_codex import responses_message
                        message, finish = responses_message(fixture, body)
                    else:
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
                    if route == "/v1/responses":
                        from acp_skill_mount_codex import responses_wire
                        response, stream = responses_wire(message, body["model"])
                        data = stream if body.get("stream") else json.dumps(response).encode()
                        content_type = "text/event-stream" if body.get("stream") else "application/json"
                    if route == "/v1/messages":
                        from acp_skill_mount_claude import messages_wire
                        response, stream = messages_wire(message, body["model"])
                        data = stream if body.get("stream") else json.dumps(response).encode()
                        content_type = "text/event-stream" if body.get("stream") else "application/json"
                    if gemini:
                        from acp_skill_mount_gemini import generate_wire
                        response, stream = generate_wire(message)
                        data = stream if action == "streamGenerateContent" else json.dumps(response).encode()
                        content_type = "text/event-stream" if action == "streamGenerateContent" else "application/json"
                    self.send_response(200)
                except (ValueError, KeyError, TypeError, AttributeError) as error:
                    with fixture.lock:
                        fixture.errors += 1
                        known = {"route or authentication", "unexpected model", "request size", "native catalog has no accessible mounted view of the skill",
                                 "native Skill result did not identify the mounted view",
                                 "fixed CLI has no reviewed compatible tool schema", "Goose native catalog does not equal selected skills", "Hermes native catalog does not equal selected skills",
                                 "unselected native skill leaked or selected skill missing",
                                 "added skill was not discovered", "removed or other-session skill leaked"}
                        category = str(error) if str(error) in known else "wire or request validation"
                        if category not in fixture.error_categories:
                            fixture.error_categories.append(category)
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
