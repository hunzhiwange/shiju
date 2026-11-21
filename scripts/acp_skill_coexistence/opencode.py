"""Exercise the pinned OpenCode scanner, Chat Completions and native stdio MCP."""
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

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

from . import correlated_result
from .cortex_code import mcp_server

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".agents/skills"
REVISION = "opencode-1.19.5-model-source-v2"
MCP_TOOL = "coexistence_read_probe"
MCP_REJECTION = "The user rejected permission to use this specific tool call."


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed OpenCode platform")
    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(manifest.read_text())["opencode"]
    require(contract["version"] == "1.19.5", "unexpected OpenCode contract version")
    relative = Path("node_modules") / contract["package"] / contract["bin"]
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = base / "acp-runtime" / REVISION / relative
    entry = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION / relative
    entry.parent.mkdir(parents=True)
    old_runtime = base / "acp-runtime/opencode-1.19.5-model-source-v1/node_modules"
    metadata = source.parent.parent / "package.json" if source.is_file() else old_runtime / contract["package"] / "package.json"
    require(metadata.is_file(), "OpenCode package identity unavailable")
    shutil.copy2(metadata, entry.parent.parent / "package.json")
    if source.is_file():
        shutil.copy2(source, entry)
    else:
        upstream = old_runtime / "opencode-darwin-arm64/bin/opencode"
        require(upstream.is_file(), "reviewed OpenCode upstream executable unavailable")
        shutil.copy2(upstream, entry)
        patch = ROOT / "patches/acp-model-source/opencode/apply-skills.py"
        try:
            result = subprocess.run([sys.executable, str(patch), str(entry)],
                                    env=env, capture_output=True, timeout=30)
        except subprocess.TimeoutExpired:
            raise AssertionError("OpenCode confined runtime build timed out") from None
        require(result.returncode == 0, "OpenCode confined runtime build failed")
    sha256 = hashlib.sha256()
    with entry.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            sha256.update(block)
    digest = sha256.hexdigest()
    require(digest == contract["files"][contract["bin"]], "OpenCode executable digest mismatch")
    try:
        signature = subprocess.run(["codesign", "--verify", "--strict", str(entry)],
                                   env=env, capture_output=True, timeout=15)
        version = subprocess.run([str(entry), "--version"], env=env, capture_output=True, timeout=15)
    except subprocess.TimeoutExpired:
        raise AssertionError("OpenCode executable verification timed out") from None
    require(signature.returncode == 0, "OpenCode signature verification failed")
    require(version.returncode == 0 and version.stdout.strip() == b"1.19.5",
            "unexpected OpenCode executable version")
    return {"command": str(entry), "args": ["acp"]}, {
        "version": contract["version"], "revision": REVISION, "sha256": digest}


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
        require(body["model"] == MODEL, "unexpected OpenCode model")
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if not tools:
            return {"content": "Fixture session"}, "stop"
        self.requests += 1
        skills = self.fixtures["skills"]
        messages = body["messages"]
        if self.requests == 1:
            envelope = json.dumps(body)
            description = tools.get("skill", {}).get("description", "")
            system = "\n".join(str(row["content"]) for row in messages if row["role"] == "system")
            catalog = description + system
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog) == visible, f"unexpected OpenCode {kind} catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "OpenCode skill content appeared before activation")
            require(self.fixtures["mcp"] not in envelope, "MCP value appeared before invocation")
            require((MCP_TOOL in tools) == self.mounted,
                    "unexpected OpenCode stdio MCP catalog")
            return self.activate(tools, "native")

        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"]) for row in messages if row["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        self.completed[step] = call_id
        if step in ("native", "selected"):
            skill = skills[step]
            require(skill["body"] in result and skill["attachment"] not in result,
                    f"unexpected OpenCode {step} activation content")
            match = re.search(r"Base directory for this skill: ([^\n]+)", result)
            require(match is not None, "OpenCode activation omitted the directory")
            directory = Path(match[1].strip())
            require(directory.is_absolute() and directory.resolve() == skill["path"].resolve(),
                    f"OpenCode {step} activated an unexpected directory")
            if step == "selected":
                require(directory.parent.parent.name.startswith("shiju-session-skills-"),
                        "OpenCode selected skill bypassed the session view")
            return self.call(tools, step + "_read", "read",
                             {"filePath": str(directory / "references/value.txt")})
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and re.search(r"reject|denied|cancel", result, re.I))
                    if self.rejected else value in result, "unexpected OpenCode MCP continuation")
        else:
            require(skills[step.removesuffix("_read")]["attachment"] in result,
                    f"OpenCode {step} did not return its attachment")
        if self.mounted and step == "native_read":
            return self.activate(tools, "selected")
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def activate(self, tools, kind):
        return self.call(tools, kind, "skill", {"name": self.fixtures["skills"][kind]["name"]})

    def call(self, tools, step, name, arguments):
        require(name in tools, f"OpenCode {step} tool missing")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= arguments.keys() <= schema.get("properties", {}).keys(),
                f"OpenCode {step} tool schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.pending = (step, name, arguments, call_id)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def observe_terminal(self, events):
        if not self.rejected or "mcp" in self.completed or self.pending is None:
            return False
        step, name, _, call_id = self.pending
        if step != "mcp":
            return False
        terminal = [event for event in events if event.get("type") == "chat.tool"
                    and event.get("call_id") == call_id and event.get("kind") == "complete"]
        if len(terminal) != 1 or terminal[0].get("status") != "failed":
            return False
        payload = terminal[0]["payload"]
        if payload.get("title") == name and payload.get("error") == MCP_REJECTION:
            self.completed["mcp"] = call_id
            return True
        return False

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == "mcp__coexistence__read_probe"
                and metadata.get("arguments") in (None, {}))


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated OpenCode model request")
            require(self.path == "/v1/chat/completions", "unexpected OpenCode model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid OpenCode model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid OpenCode Chat Completions envelope")
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
