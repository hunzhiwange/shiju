"""Probe the fixed Darwin aionrs v2 bridge through the production gateway."""

import hashlib
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

from . import correlated_result

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".aionrs/skills"
VERSION = "0.2.11"
REVISION = f"aionrs-{VERSION}-model-source-v2"
SOURCE_SHA256 = "102f0682858a4f02b505b0413578a0de508a1837cd7b93e5626643d63dd06688"
MCP_TOOL = "mcp__coexistence__read_probe"
DIGESTS = {
    "aionrs": "aae767bf2d5132d730a77526132fe3c689ff2cf60c81284d0b7a86f97ffa1662",
    "aionrs-acp": "b2ab720e864645b1b0dc681fcf852873c501d3a8f19de7b9a1ffc2408c9a466a",
}


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed aionrs platform")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"
                           / "contracts.json").read_text())["aionrs"]
    require(contract["version"] == VERSION and contract["bin"] == "aionrs-acp"
            and contract["source_sha256"] == SOURCE_SHA256,
            "unexpected aionrs contract")
    source = Path(os.environ.get("ACP_AIONRS_REVIEWED_DIR",
                                 Path.home() / ".local/share/shiju/acp-runtime" / REVISION))
    target = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    target.mkdir(parents=True, mode=0o700)
    for name, expected in DIGESTS.items():
        artifact = target / name
        shutil.copy2(source / name, artifact)
        require(hashlib.sha256(artifact.read_bytes()).hexdigest() == expected,
                f"reviewed {name} artifact digest mismatch")
    (target / "build-receipt.json").write_text(json.dumps({
        "version": VERSION, "source_sha256": contract["source_sha256"],
        "sha256": DIGESTS["aionrs-acp"], "engine_sha256": DIGESTS["aionrs"],
    }))
    version = subprocess.run([str(target / "aionrs"), "--version"], env=env,
                             capture_output=True, timeout=15)
    require(version.returncode == 0 and version.stdout.strip() == b"aionrs 0.2.11",
            "unexpected aionrs engine version")
    return {"command": str(target / "aionrs-acp"), "args": []}, {
        "version": VERSION, "revision": REVISION, "platform": "Darwin-arm64",
        "source_sha256": contract["source_sha256"], "sha256": DIGESTS,
    }


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.failure = None
        self.pending = None

    def response(self, body):
        require(body["model"] == MODEL and body.get("stream") is True,
                "unexpected aionrs Chat Completions contract")
        self.requests += 1
        tools = {row["function"]["name"]: row["function"] for row in body.get("tools", [])}
        require({"Read", "Write", "Edit", "Glob"} <= tools.keys(),
                "aionrs workspace tool catalog is incomplete")
        skills = self.fixtures["skills"]
        if self.requests == 1:
            system = "\n".join(str(item.get("content", "")) for item in body["messages"]
                               if item["role"] == "system")
            envelope = json.dumps(body)
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                listed = re.search(rf"(?m)^- {re.escape(skill['name'])}: ", system) is not None
                require(listed == visible, f"unexpected aionrs {kind} skill catalog")
                require(all(skill[key] not in envelope for key in ("body", "attachment")),
                        "aionrs skill content appeared before activation")
            require(self.fixtures["mcp"] not in envelope, "aionrs MCP value appeared before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected aionrs MCP catalog")
            return self.activate(tools, "native", skills["native"])

        step, _, call_id = self.pending
        results = [dict(message, tool_use_id=message["tool_call_id"])
                   for message in body["messages"] if message["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        kind = step.removesuffix("_read")
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and re.search(r"denied|reject", result, re.I))
                    if self.rejected else value in result, "unexpected aionrs MCP continuation")
        else:
            skill = skills[kind]
            value = skill["attachment"] if step.endswith("_read") else skill["body"]
            require(value in result, f"missing aionrs {step} content")
        self.completed[step] = call_id

        if step in ("native", "selected"):
            base = skills[step]["path"]
            if step == "selected":
                match = re.search(r"Base directory for this skill: ([^\r\n]+)", result)
                require(match is not None, "aionrs selected activation omitted the private path")
                base = Path(match[1].strip())
                require(base.is_absolute() and base.name == skills[step]["name"]
                        and base != skills[step]["path"], "aionrs did not use a private skill view")
            return self.call(tools, step + "_read", "Read", {"file_path": str(base / "references/value.txt")})
        if self.mounted and step == "native_read":
            return self.activate(tools, "selected", skills["selected"])
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def activate(self, tools, step, skill):
        require("Skill" in tools, "aionrs native catalog is visible but the managed tool list omits Skill")
        return self.call(tools, step, "Skill", {"skill": skill["name"]})

    def call(self, tools, step, name, arguments):
        schema = tools[name]["parameters"]
        require(set(schema.get("required", ())) <= arguments.keys() <= schema.get("properties", {}).keys(),
                f"unexpected aionrs {step} tool schema")
        call_id = "call_" + secrets.token_hex(12)
        self.pending = (step, name, call_id)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def approves(self, metadata):
        return (self.pending is not None and self.pending[:2] == ("mcp", MCP_TOOL)
                and metadata.get("title") == MCP_TOOL and metadata.get("arguments") == {})

    def approves_skill(self, metadata):
        if self.pending is not None:
            step, name, _ = self.pending
            if step != "mcp" and metadata.get("title") == name and metadata.get("arguments") == {}:
                return step

    def expected_skill_approvals(self):
        return ["native", "native_read", "selected", "selected_read"] if self.mounted else [
            "native", "native_read"]


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.path == "/v1/chat/completions", "unexpected aionrs model protocol")
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated aionrs model request")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid aionrs model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid aionrs model envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.chunk(body["model"], dict(delta, role="assistant"), None)
        self.chunk(body["model"], {}, finish)
        self.wfile.write(b"data: [DONE]\n\n")
