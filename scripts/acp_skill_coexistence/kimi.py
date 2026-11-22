"""Probe the fixed Kimi ACP's native Skill, Read and session stdio MCP tools."""

import hashlib
from html import unescape
import json
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".kimi-code/skills"
VERSION = "2.5.0"
SHA256 = "d4dd02d40f18f3a784699fc37ae0c55d676a4704c37a3b91ab96541b0db673a1"
ARTIFACT = ROOT / "target/acp-skill-coexistence/kimi-artifact/kimi"
CHAT_ENDPOINT = "/v1/chat/completions"
MCP_TOOL = "mcp__coexistence__read_probe"


def launch(env):
    system, machine = platform.system(), platform.machine()
    artifact = ARTIFACT
    version, expected_digest = VERSION, SHA256
    args = ["acp"]
    evidence = {}
    if system == "Linux":
        manifest = json.loads((ROOT / "patches/acp-model-source/kimi/linux.json").read_text())
        target = manifest["targets"].get(machine)
        require(target is not None, "unreviewed Linux Kimi architecture")
        artifact = ARTIFACT.parent / target["platform"] / "kimi"
        version, expected_digest = manifest["version"], target["sha256"]
        args = manifest["args"]
        evidence = {"target": target["platform"], "args": args,
                    "model_endpoint": CHAT_ENDPOINT}
    else:
        require((system, machine) == ("Darwin", "arm64"), "unreviewed Kimi platform")
    require(artifact.is_file(), "missing pinned Kimi executable at " + str(artifact.relative_to(ROOT)))
    entry = Path(env["TMPDIR"]) / "kimi"
    shutil.copy2(artifact, entry)
    checksum = hashlib.sha256()
    with entry.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(block)
    digest = checksum.hexdigest()
    require(digest == expected_digest, "Kimi executable digest mismatch")
    try:
        readback = subprocess.run([str(entry), "--version"], env=env, capture_output=True, timeout=15)
    except subprocess.TimeoutExpired as error:
        raise AssertionError("Kimi version readback timed out") from error
    require(readback.returncode == 0 and readback.stdout.strip() == version.encode(),
            "unexpected Kimi executable version")
    return {"command": str(entry), "args": args}, {"version": version, "sha256": digest, **evidence}


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.pending = None
        self.failure = None

    def response(self, body):
        require(body["model"] == MODEL and body.get("stream") is True,
                "unexpected Kimi Chat Completions contract")
        tools = {row["function"]["name"]: row["function"] for row in body.get("tools", [])}
        if not tools:
            return {"content": "Fixture session"}, "stop"
        self.requests += 1
        messages = body["messages"]
        envelope = json.dumps(messages)
        skills = self.fixtures["skills"]
        if self.requests == 1:
            require((MCP_TOOL in tools) == self.mounted, "unexpected Kimi stdio MCP catalog")
            require({"Skill", "Read"} <= tools.keys(), "Kimi native Skill or Read missing")
            catalogs = [str(row.get("content")) for row in messages
                        if "Current available skills:" in str(row.get("content"))]
            require(catalogs, "Kimi native skill listing missing")
            for kind, skill in skills.items():
                visible = kind == "native" or (self.mounted and kind == "selected")
                require(any(f'- {skill["name"]}:' in catalog for catalog in catalogs) == visible,
                        f"unexpected Kimi {kind} skill visibility")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Kimi skill content appeared before activation")
            require(self.fixtures["mcp"] not in envelope, "MCP value appeared before invocation")
            return self.call(tools, "native", "Skill", {"skill": skills["native"]["name"]})

        step, name, args, call_id = self.pending
        calls = [call for row in messages if row["role"] == "assistant"
                 for call in row.get("tool_calls", []) if call["id"] == call_id]
        require(len(calls) == 1 and calls[0]["function"]["name"] == name
                and json.loads(calls[0]["function"]["arguments"]) == args,
                "Kimi continuation omitted the assistant tool call")
        results = [str(row.get("content")) for row in messages
                   if row["role"] == "tool" and row.get("tool_call_id") == call_id]
        require(len(results) == 1, "missing or ambiguous Kimi tool result")
        result = results[0]

        if step in ("native", "selected"):
            skill = skills[step]
            require(f'Skill "{skill["name"]}" loaded inline.' in result,
                    "Kimi Skill did not acknowledge activation")
            loaded = [match for row in messages if row["role"] == "user"
                      for match in re.finditer(r'<skill-loaded name="' + re.escape(skill["name"])
                                                + r'" trigger="model-tool"[^>]* dir="([^"]+)"[^>]*>'
                                                  r'(.*?)</skill-loaded>', str(row.get("content")), re.S)]
            require(len(loaded) == 1 and skill["body"] in loaded[0][2],
                    "Kimi native activation omitted the fixture body")
            base = Path(unescape(loaded[0][1]))
            require(base.is_absolute() and base.name == skill["name"], "invalid Kimi skill path")
            if step == "native":
                require(base == skill["path"], "unexpected Kimi native path")
            else:
                require(base != skill["path"] and base.resolve() == skill["path"].resolve()
                        and any(part.startswith("shiju-session-skills-") for part in base.parts),
                        "Kimi selected skill bypassed its session view")
            self.completed[step] = call_id
            return self.call(tools, step + "_read", "Read", {"path": str(base / "references/value.txt")})
        if step.endswith("_read"):
            require(skills[step.removesuffix("_read")]["attachment"] in result,
                    "Kimi Read did not return the fixture attachment")
            self.completed[step] = call_id
        if step == "native_read" and self.mounted:
            return self.call(tools, "selected", "Skill", {"skill": skills["selected"]["name"]})
        if step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and re.search(r"reject|denied|cancel", result, re.I))
                    if self.rejected else value in result, "unexpected Kimi MCP outcome")
            self.completed[step] = call_id
        else:
            require(MCP_TOOL not in tools and MCP_TOOL not in envelope,
                    "unselected Kimi MCP tool remained visible")
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def call(self, tools, step, name, args):
        require(name in tools, f"Kimi {name} tool missing")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= args.keys() <= schema.get("properties", {}).keys(),
                f"unexpected Kimi {name} tool schema")
        call_id = "call_" + secrets.token_hex(12)
        self.pending = step, name, args, call_id
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(args)}}]}, "tool_calls"

    def approves(self, metadata):
        # Kimi's permission update sends its exact tool title without raw input.
        return self.pending is not None and self.pending[:3] == ("mcp", MCP_TOOL, {}) \
            and metadata.get("title") == MCP_TOOL and metadata.get("arguments") in (None, {})


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.path == CHAT_ENDPOINT, "unexpected Kimi model protocol")
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Kimi model request")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Kimi model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Kimi model envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.chunk(body["model"], dict(delta, role="assistant"), None)
        self.chunk(body["model"], {}, finish)
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()
