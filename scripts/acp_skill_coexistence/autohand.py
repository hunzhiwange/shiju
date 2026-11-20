"""Probe AutoHand's original ACP, local skill tools, and stdio MCP."""

import hashlib
import json
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess

from verify_acp_integration import ProviderHandler, require

from . import correlated_result

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".autohand/skills"
# AutoHand checks the inherited cwd before receiving ACP session/new.
GATEWAY_WORKSPACE_CWD = True
MCP_TOOL = "mcp__coexistence__read_probe"
PROMPT = ("Inspect and activate the available fixture skill, read its file reference, "
          "and check whether the mcp__coexistence__read_probe MCP tool is available.")
VERSION = "0.9.11 (46a28b16)"
SHA256 = "339bdb0c8f4f8b7f9076108434d910b2cd5e14f2593b48c0e5858e04e91950eb"
DIRECT_ARGS = ["--acp", "--set", "features.codeMode=false",
               "--set", "ui.completionReportEnabled=false"]


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed AutoHand platform")
    command = shutil.which("autohand", path=env["PATH"])
    require(command is not None, "reviewed AutoHand entry unavailable")
    entry = Path(command).resolve(strict=True)
    require(hashlib.sha256(entry.read_bytes()).hexdigest() == SHA256,
            "AutoHand entry digest mismatch")
    metadata = json.loads((entry.parent.parent / "package.json").read_text())
    require((metadata["name"], metadata["version"]) == ("autohand-cli", "0.9.11"),
            "unexpected AutoHand package identity")
    version = subprocess.run([str(entry), "--version"], env=env, capture_output=True,
                             text=True, timeout=10)
    require(version.returncode == 0 and VERSION in version.stdout,
            "unexpected AutoHand executable version")
    return {"command": command, "args": DIRECT_ARGS, "env": {
        "AUTOHAND_SKIP_AHTRACES_INSTALL": "1",
        "AUTOHAND_SKIP_COMPUTER_CONTROL_INSTALL": "1",
    }}, {
        "version": VERSION, "sha256": SHA256, "tool_mode": "reviewed direct tools",
    }


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.pending = None
        self.locations = {}
        self.failure = None

    def response(self, body):
        require(body["model"] == MODEL, "unexpected AutoHand upstream model")
        tools = {row["function"]["name"]: row["function"] for row in body.get("tools", [])}
        if not tools:
            return {"content": "Fixture session"}, "stop"
        self.requests += 1
        skills = self.fixtures["skills"]
        if self.pending is None:
            system_parts = []
            for row in body["messages"]:
                if row["role"] != "system":
                    continue
                content = row["content"]
                if isinstance(content, str):
                    system_parts.append(content)
                else:
                    system_parts.extend(part["text"] for part in content if part["type"] == "text")
            system = "\n".join(system_parts)
            catalog = re.search(r"(?m)^## Available Skills\n(.*?)(?=^## |\Z)", system, re.S)
            require(catalog is not None, "AutoHand native catalog missing")
            envelope = json.dumps(body)
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog[1]) == visible,
                        f"unexpected AutoHand {kind} skill catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "AutoHand skill content appeared before activation")
            require(self.fixtures["mcp"] not in envelope, "MCP value appeared before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected AutoHand MCP catalog")
            return self.skill_call(tools, "native", "info")

        step, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"]) for row in body["messages"]
                   if row["role"] == "tool"]
        result = correlated_result(results, call_id)["content"]
        if step.endswith("_info"):
            kind = step.removesuffix("_info")
            info = json.loads(result)
            expected = skills[kind]["path"] / "SKILL.md"
            path = Path(info["path"])
            require(info["name"] == skills[kind]["name"] and not info["active"]
                    and path.is_absolute() and path.resolve() == expected.resolve(),
                    f"unexpected AutoHand {kind} skill location")
            if kind == "selected":
                require(any(part.startswith("shiju-session-skills-") for part in (*path.parts, *path.resolve().parts)),
                        "AutoHand selected skill bypassed the session view")
            self.locations[kind] = path
            return self.skill_call(tools, kind, "activate")
        if step in self.locations:
            skill = skills[step]
            require(f"Activated skill: {skill['name']}" in result,
                    f"AutoHand did not activate the {step} skill")
            return self.call(tools, step + "_body", "read_file",
                             {"path": str(self.locations[step])})
        if step.endswith("_body"):
            kind = step.removesuffix("_body")
            require(skills[kind]["body"] in result, f"unexpected AutoHand {kind} body")
            self.completed[kind] = call_id
            return self.read_reference(tools, kind)

        if step == "mcp_search":
            require(MCP_TOOL in result, "AutoHand tool search did not find the stdio MCP tool")
            return self.call(tools, "mcp", MCP_TOOL, {})
        if step == "mcp":
            token = self.fixtures["mcp"]
            require((token not in result and re.search(r"reject|denied|cancel", result, re.I))
                    if self.rejected else token in result, "unexpected AutoHand MCP continuation")
        else:
            kind = step.removesuffix("_read")
            require(skills[kind]["attachment"] in result, f"unexpected AutoHand {step} result")
        self.completed[step] = call_id
        if self.mounted and step == "native_read":
            return self.skill_call(tools, "selected", "info")
        if step == "selected_read":
            if MCP_TOOL not in tools:
                return self.call(tools, "mcp_search", "tool_search", {"query": MCP_TOOL})
            return self.call(tools, "mcp", MCP_TOOL, {})
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def read_reference(self, tools, kind):
        path = self.locations[kind].parent / "references/value.txt"
        return self.call(tools, kind + "_read", "read_file", {"path": str(path)})

    def skill_call(self, tools, kind, command):
        step = kind + "_info" if command == "info" else kind
        return self.call(tools, step, "skill",
                         {"command": command, "name": self.fixtures["skills"][kind]["name"]})

    def call(self, tools, step, name, args):
        require(name in tools, f"AutoHand {step} tool missing")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= args.keys() <= schema.get("properties", {}).keys(),
                f"unexpected AutoHand {step} tool schema")
        call_id = "probe_" + secrets.token_hex(12)
        self.pending = step, call_id
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(args)}}]}, "tool_calls"

    def observe_terminal(self, events):
        if self.failure:
            return False
        for event in events:
            if event.get("type") != "chat.error":
                continue
            error = str(event.get("error", ""))
            if "no verified skill directory mount branch" in error:
                self.failure = "AutoHand session view rejected at the host capability gate"
            elif "Autohand Code could not complete this turn" in error:
                self.failure = "AutoHand ACP prompt failed before all correlated reads"
            elif "acp initialize failed" in error:
                self.failure = "AutoHand ACP initialization failed"
        return False

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == MCP_TOOL and metadata.get("arguments") == {})


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated AutoHand model request")
            require(self.path == "/v1/chat/completions", "unexpected AutoHand model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid AutoHand model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid AutoHand Chat Completions envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        message = dict(delta, role="assistant")
        if body.get("stream"):
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            self.chunk(body["model"], message, None)
            self.chunk(body["model"], {}, finish)
            self.wfile.write(b"data: [DONE]\n\n")
        else:
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"id": "chatcmpl-fixture", "object": "chat.completion",
                "model": body["model"], "choices": [{"index": 0, "finish_reason": finish,
                                                    "message": message}]}).encode())
