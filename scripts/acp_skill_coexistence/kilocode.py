"""Probe Kilocode 7.8.18 over OpenAI Chat Completions and ACP stdio MCP."""
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
from .cortex_code import mcp_server

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".agents/skills"
MCP_TOOL = "coexistence_read_probe"
MCP_REJECTION = "The user rejected permission to use this specific tool call."


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Kilocode platform")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json")
                          .read_text())["kilocode"]
    require(contract["version"] == "7.8.18", "unexpected Kilocode contract version")
    revision = "kilocode-7.8.18-model-source-v2"
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    relative = Path("node_modules") / contract["package"] / contract["bin"]
    runtime = base / "acp-runtime"
    source = runtime / revision / relative
    package_path = source.parent.parent / "package.json"
    if override := os.environ.get("ACP_COEXISTENCE_KILOCODE_BIN"):
        source = Path(override)
        # The pinned upstream package metadata is identical across repacks.
        if not package_path.is_file():
            package_path = runtime / "kilocode-7.8.18-model-source-v1" / relative.parent.parent / "package.json"
    package = json.loads(package_path.read_text())
    require(package["name"] == contract["package"] and package["version"] == contract["version"],
            "unexpected Kilocode package identity")
    require(source.is_file(), "reviewed Kilocode executable unavailable")
    entry = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision / relative
    entry.parent.mkdir(parents=True)
    shutil.copy2(source, entry)
    shutil.copy2(package_path, entry.parent.parent / "package.json")
    sha256 = hashlib.sha256()
    with entry.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            sha256.update(block)
    digest = sha256.hexdigest()
    require(digest == contract["files"][contract["bin"]], "Kilocode executable digest mismatch")
    try:
        signature = subprocess.run(["codesign", "--verify", "--strict", str(entry)],
                                   env=env, capture_output=True, timeout=15)
        version = subprocess.run([str(entry), "--version"], env=env, capture_output=True, timeout=15)
    except subprocess.TimeoutExpired:
        raise AssertionError("Kilocode executable verification timed out") from None
    require(signature.returncode == 0, "Kilocode signature verification failed")
    require(version.returncode == 0 and version.stdout.strip() == b"7.8.18",
            "unexpected Kilocode executable version")
    return {"command": str(entry), "args": ["acp", "--pure"]}, {
        "version": contract["version"], "revision": revision, "sha256": digest,
    }


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.calls = {}
        self.completed = {}
        self.failure = None
        self.pending = None

    def response(self, body):
        require(body["model"] == MODEL, "unexpected Kilocode model")
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if not tools:
            return {"content": "Fixture session"}, "stop"
        self.requests += 1
        messages = body["messages"]
        skills = self.fixtures["skills"]
        if self.requests == 1:
            system = "\n".join(str(row["content"]) for row in messages if row["role"] == "system")
            catalogs = "\n".join(re.findall(r"<available_skills>(.*?)</available_skills>", system, re.S))
            names = set(re.findall(r"<name>\s*([^<\s]+)\s*</name>", catalogs))
            envelope = json.dumps(body)
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in names) == visible, f"unexpected Kilocode {kind} skill catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Kilocode skill content appeared before activation")
            require(self.fixtures["mcp"] not in envelope, "MCP fixture leaked before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected Kilocode stdio MCP catalog")
        results = [dict(row, tool_use_id=row["tool_call_id"]) for row in messages if row["role"] == "tool"]
        for kind in ("native", "selected") if self.mounted else ("native",):
            skill = skills[kind]
            if kind not in self.calls:
                return self.call(tools, kind, "skill", {"name": skill["name"]})
            activation = str(correlated_result(results, self.calls[kind], skill["body"])["content"])
            self.completed[kind] = self.calls[kind]
            step = kind + "_read"
            if step not in self.calls:
                match = re.search(r"Base directory for this skill: ([^\n]+)", activation)
                require(match is not None, "Kilocode activation omitted its base directory")
                directory = Path(match[1].strip().removeprefix("file://"))
                require(directory.is_absolute() and directory.resolve() == skill["path"].resolve(),
                        "Kilocode activated an unexpected skill directory")
                if kind == "selected":
                    require(directory.parent.parent.name.startswith("shiju-session-skills-"),
                            "Kilocode bypassed the session skill view")
                return self.call(tools, step, "read", {"filePath": str(directory / "references/value.txt")})
            correlated_result(results, self.calls[step], skill["attachment"])
            self.completed[step] = self.calls[step]
        if self.mounted:
            if "mcp" not in self.calls:
                return self.call(tools, "mcp", MCP_TOOL, {})
            result = str(correlated_result(results, self.calls["mcp"])["content"])
            if self.rejected:
                require(re.search(r"reject|denied|cancel", result, re.I)
                        and self.fixtures["mcp"] not in result, "Kilocode MCP rejection missing")
            else:
                require(self.fixtures["mcp"] in result, "Kilocode MCP result lacks fixture value")
            self.completed["mcp"] = self.calls["mcp"]
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def call(self, tools, step, name, arguments):
        require(name in tools, f"Kilocode {step} tool missing")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= arguments.keys() <= schema.get("properties", {}).keys(),
                f"Kilocode {step} tool schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.calls[step] = call_id
        self.pending = step
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def observe_terminal(self, events):
        if not self.rejected or self.pending != "mcp" or "mcp" in self.completed:
            return False
        call_id = self.calls["mcp"]
        failures = [event for event in events if event.get("type") == "chat.tool"
                    and event.get("call_id") == call_id and event.get("kind") == "complete"
                    and event.get("status") == "failed"]
        if len(failures) == 1:
            payload = failures[0]["payload"]
            if payload.get("title") == MCP_TOOL and payload.get("error") == MCP_REJECTION:
                self.completed["mcp"] = call_id
                return True
        return False

    def approves(self, metadata):
        return (self.pending == "mcp" and metadata.get("title") == "mcp__coexistence__read_probe"
                and metadata.get("arguments") == {})


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Kilocode model request")
            require(self.path == "/v1/chat/completions", "unexpected Kilocode model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Kilocode model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Kilocode Chat Completions envelope")
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
