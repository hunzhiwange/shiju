"""Probe AutoHand's original ACP, local skill tools, and stdio MCP."""

import hashlib
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess
import sys
import tempfile

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT as REPOSITORY

from . import correlated_result
from .mimo_code import mcp_server

MODEL = "deepseek-v4.1-flash-1017"
# The pinned OpenAI client requests this budget even for a short tool turn.
MODEL_OUTPUT_LIMIT = 16_000
ADAPTER = "openai"
API_PATH = "/v1"
MODEL_ENDPOINT = API_PATH + "/chat/completions"
TRANSPORT = "stdio"
NATIVE_DIR = ".autohand/skills"
GLOBAL_SKILL_FIXTURES = True
# AutoHand checks the inherited cwd before receiving ACP session/new.
GATEWAY_WORKSPACE_CWD = True
MCP_TOOL = "mcp__coexistence__read_probe"
PROMPT = ("Inspect and activate the available fixture skill, read its file reference, "
          "and check whether the mcp__coexistence__read_probe MCP tool is available.")
# AutoHand can prepare these native specialists before the primary session prompt.
SPECIALIST_PROMPTS = (
    "You are a read-only security auditor.",
    "# Autohand Review",
    "You are a software delivery planner.",
    "You are a verification engineer.",
)
VERSION = "0.9.11 (46a28b16)"
SHA256 = "339bdb0c8f4f8b7f9076108434d910b2cd5e14f2593b48c0e5858e04e91950eb"
DIRECT_ARGS = ["--acp", "--set", "features.codeMode=false",
               "--set", "ui.completionReportEnabled=false"]


def launch(env):
    target = (platform.system(), platform.machine())
    require(target in {("Darwin", "arm64"), ("Linux", "x86_64"), ("Linux", "aarch64")},
            "unsupported AutoHand probe platform")
    registry = REPOSITORY / "crates/libs/acp-agent-registry/src/managed_runtime"
    manifest = registry / "contracts.json"
    contract = json.loads(manifest.read_text())["autohand"]
    revision = "autohand-0.9.11-model-source-v1"
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision
    runtime.parent.mkdir(mode=0o700)
    source = os.environ.get("AUTOHAND_COEXISTENCE_RUNTIME")
    if source:
        shutil.copytree(Path(source).resolve(strict=True), runtime, symlinks=True)
    else:
        with tempfile.TemporaryDirectory(dir=env["TMPDIR"]) as stage:
            assets = Path(stage)
            patch = REPOSITORY / "patches/acp-model-source/autohand"
            shutil.copy2(manifest, assets)
            shutil.copytree(patch, assets / "autohand")
            shutil.copy2(patch / "package-lock.json", assets / "autohand-lock.json")
            try:
                install = subprocess.run(
                    [sys.executable, str(registry / "install.py"), stage, str(runtime), "autohand"],
                    env=env, capture_output=True, timeout=180,
                )
            except subprocess.TimeoutExpired:
                raise AssertionError("reviewed AutoHand installation timed out") from None
            require(install.returncode == 0, "reviewed AutoHand installation failed")
    package = runtime / "node_modules" / contract["package"]
    files = {name: hashlib.sha256((package / name).read_bytes()).hexdigest()
             for name in contract["files"]}
    require(files == contract["files"] and files[contract["bin"]] == SHA256,
            "AutoHand runtime digest mismatch")
    entry = runtime / "node_modules/.bin/autohand"
    require(entry.resolve(strict=True) == (package / contract["bin"]).resolve(strict=True),
            "unexpected AutoHand executable target")
    version = subprocess.run([str(entry), "--version"], env=env, capture_output=True,
                             text=True, timeout=15)
    require(version.returncode == 0 and VERSION in version.stdout,
            "unexpected AutoHand executable version")
    return {"command": str(entry), "args": DIRECT_ARGS, "env": {
        "AUTOHAND_SKIP_AHTRACES_INSTALL": "1",
        "AUTOHAND_SKIP_COMPUTER_CONTROL_INSTALL": "1",
    }}, {"version": VERSION, "revision": revision, "entry": contract["bin"],
         "model_endpoint": MODEL_ENDPOINT, "sha256": files}


class Probe:
    def __init__(self, fixtures, mounted, rejected, *, observe_stdio=True):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.pending = None
        self.locations = {}
        self.failure = None
        self.observe_stdio = observe_stdio
        self.audit = None

    def response(self, body):
        require(body["model"] == MODEL, "unexpected AutoHand upstream model")
        tools = {row["function"]["name"]: row["function"] for row in body.get("tools", [])}
        if not tools:
            return {"content": "Fixture session"}, "stop"
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
            envelope = json.dumps(body)
            if catalog is None and system.startswith(SPECIALIST_PROMPTS):
                require(MCP_TOOL not in tools and self.fixtures["mcp"] not in envelope
                        and all(skill["body"] not in envelope and skill["attachment"] not in envelope
                                for skill in skills.values()),
                        "AutoHand specialist received a private fixture resource")
                return {"content": "No additional work is required for this fixture."}, "stop"
            require(catalog is not None, "AutoHand native catalog missing")
            self.requests += 1
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog[1]) == visible,
                        f"unexpected AutoHand {kind} skill catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "AutoHand skill content appeared before activation")
            require(self.fixtures["mcp"] not in envelope, "MCP value appeared before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected AutoHand MCP catalog")
            if self.mounted and self.observe_stdio:
                root = skills["native"]["path"].parents[3]
                audits = list((root / "tmp").glob("shiju-mimo-mcp-*/mimo-fixture-audit"))
                require(len(audits) == 1, "missing or ambiguous AutoHand stdio audit")
                self.audit = root / ("autohand-rejected-audit" if self.rejected else "autohand-mounted-audit")
                os.link(audits[0], self.audit)
                require(not self.audit.read_text(), "AutoHand MCP ran before approval")
            return self.skill_call(tools, "native", "info")

        self.requests += 1
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
                    and path.is_absolute() and path.name == "SKILL.md",
                    f"unexpected AutoHand {kind} skill location")
            if kind == "native":
                require(path.resolve() == expected.resolve(), "unexpected AutoHand native source")
            else:
                require(path.parent.name == skills[kind]["name"]
                        and path.parent.parent.name.startswith("shiju-session-skills-")
                        and path.resolve() == path,
                        "AutoHand selected skill bypassed the request snapshot")
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
            require((token not in result and "Tool execution skipped by user." in result)
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

    def mcp_trace(self, _events):
        return (self.audit.read_text().splitlines() if self.audit else [],
                self.completed.get("mcp"))


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated AutoHand model request")
            require(self.path == MODEL_ENDPOINT, "unexpected AutoHand model protocol")
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
