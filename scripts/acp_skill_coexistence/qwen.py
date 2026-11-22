"""Qwen 0.26.7 OpenAI Chat Completions and native skill/MCP probe."""

import hashlib
from http.server import BaseHTTPRequestHandler
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
import time

from verify_acp_integration import require
from verify_acp_model_source import ROOT

MODEL = "qwen3-coder-plus"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".qwen/skills"
MCP_TOOL = "mcp__coexistence__read_probe"


def launch(env):
    system, machine = platform.system(), platform.machine()
    linux = system == "Linux" and machine in ("x86_64", "aarch64")
    require(linux or (system, machine) == ("Darwin", "arm64"), "unreviewed Qwen platform")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json")
                          .read_text())["qwen"]
    require(contract["version"] == "0.26.7", "Qwen version differs from coexistence pin")
    require(shutil.which("node", path=env["PATH"]), "Node is unavailable")
    revision = "qwen-0.26.7-model-source-v2"
    args = ["--acp"]
    if linux:
        manifest = json.loads((ROOT / "patches/acp-model-source/qwen/linux.json").read_text())
        require(all(manifest[left] == contract[right] for left, right in (
            ("package", "package"), ("version", "version"), ("entry", "bin"),
            ("integrity", "integrity"),
        )), "Qwen Linux manifest differs from the reviewed package contract")
        target = f"{machine}-unknown-linux-gnu"
        native_files = manifest["targets"][target]["files"]
        revision = manifest["managed_revision"]
        args = manifest["args"]
    if source := os.environ.get("QWEN_COEXISTENCE_RUNTIME"):
        runtime = Path(source).resolve()
    else:
        config = os.environ.get("VIBEWINDOW_CONFIG_DIR")
        base = Path(config) if config else Path.home() / ".local/share/shiju"
        runtime = base / "acp-runtime" / revision
    require((runtime / "node_modules" / contract["package"]).is_dir(),
            "reviewed Qwen package is unavailable")
    private = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision
    shutil.copytree(runtime, private, symlinks=True)
    package = private / "node_modules" / contract["package"]
    metadata = json.loads((package / "package.json").read_text())
    require((metadata["name"], metadata["version"]) == (contract["package"], contract["version"]),
            "Qwen package identity differs from contract")
    for name, digest in contract["files"].items():
        require(hashlib.sha256((package / name).read_bytes()).hexdigest() == digest,
                "Qwen artifact digest differs from contract")
    entry = package / contract["bin"]
    artifact = {"version": contract["version"], "sha256": contract["files"]}
    if linux:
        require(entry.resolve(strict=True) == entry and os.access(entry, os.X_OK),
                "Qwen Linux entry is redirected or not executable")
        verified = {}
        for name, digest in native_files.items():
            path = package / name
            require(path.resolve(strict=True) == path and path.is_file(),
                    "Qwen Linux helper is redirected or missing")
            require(name.endswith(".so") or os.access(path, os.X_OK),
                    "Qwen Linux helper is not executable")
            verified[name] = hashlib.sha256(path.read_bytes()).hexdigest()
            require(verified[name] == digest, "Qwen Linux helper digest differs from contract")
        artifact.update(target=target, native_sha256=verified,
                        model_protocol=manifest["model_protocol"])
    return {"command": str(entry), "args": args, "env": {}}, artifact


def prepare_workspace(workspace):
    # Exercise the native ACP approval path for just this MCP tool. Qwen may
    # otherwise resolve a default ask solely through its model classifier.
    (workspace / ".qwen/settings.json").write_text(json.dumps({
        "permissions": {"ask": [MCP_TOOL]},
    }))


def result_text(value):
    """Flatten Qwen's JSON-wrapped function response for content checks."""
    if isinstance(value, str):
        if value.lstrip().startswith(("{", "[")):
            try:
                return result_text(json.loads(value))
            except ValueError:
                pass
        return value
    if isinstance(value, dict):
        value = value.values()
    elif not isinstance(value, list):
        return ""
    return "\n".join(map(result_text, value))


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.calls = {}
        self.completed = {}
        self.failure = None
        self.pending_action = None
        self.search_attempts = 0

    def response(self, body):
        require(body["model"] == MODEL, "unexpected upstream model")
        messages = body["messages"]
        system = messages[0].get("content")
        if not body.get("tools") or system == (
            "You are now acting as the managed memory extraction subagent for an AI coding assistant."
        ):
            return {"role": "assistant", "content": "Ready"}, "stop"
        require(self.failure is None, self.failure)
        tools = {tool["function"]["name"]: tool["function"] for tool in body["tools"]}
        if not body.get("stream"):
            return self.classifier_response(tools, messages)
        self.requests += 1
        results = [message for message in messages if message["role"] == "tool"]
        if self.requests == 1:
            request_text = json.dumps([messages, tools])
            metadata = "\n".join(str(message.get("content", "")) for message in messages
                                 if message["role"] in ("system", "developer", "user"))
            catalog = "\n".join(re.findall(r"<available_skills>(.*?)</available_skills>",
                                           metadata, re.S))
            for kind, skill in self.fixtures["skills"].items():
                discovered = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog) == discovered, f"unexpected {kind} skill catalog")
                require(skill["body"] not in request_text and skill["attachment"] not in request_text,
                        "skill content appeared before activation")
            require(self.fixtures["mcp"] not in request_text, "MCP value appeared before invocation")
            require(MCP_TOOL not in tools, "MCP tool was advertised outside native ToolSearch")

        for kind in ("native", "selected") if self.mounted else ("native",):
            skill = self.fixtures["skills"][kind]
            if kind not in self.calls:
                return self.call(tools, kind, "skill", {"skill": skill["name"]})
            expansion = self.result(results, kind, skill["body"])
            read = kind + "_read"
            if read not in self.calls:
                match = re.search(r"Base directory for this skill: ([^\n]+)", expansion)
                require(match is not None, "native activation omitted the skill directory")
                base = Path(match[1])
                if kind == "native":
                    require(base == skill["path"], "unexpected project skill directory")
                else:
                    require(base != skill["path"] and base.name == skill["name"] and
                            base.is_absolute() and
                            (base / "SKILL.md").resolve() == (skill["path"] / "SKILL.md").resolve(),
                            "selected skill bypassed the session view")
                return self.call(tools, read, "read_file", {"file_path": str(base / "references/value.txt")})
            self.result(results, read, skill["attachment"])

        if "search" not in self.calls:
            return self.search(tools)
        search = self.result(results, "search", MCP_TOOL)
        declarations = [json.loads(item) for item in re.findall(r"<function>(.*?)</function>", search, re.S)]
        declaration = next((item for item in declarations if item["name"] == MCP_TOOL), None)
        if not self.mounted:
            require(declaration is None and f"Not found: {MCP_TOOL}" in search,
                    "unselected MCP tool was discoverable")
        else:
            if "mcp" not in self.calls:
                # Native stdio discovery can finish after the first model request.
                if f"Not found: {MCP_TOOL}" in search and self.search_attempts < 5:
                    time.sleep(0.25)
                    return self.search(tools)
                require(declaration is not None and declaration["serverName"] == "coexistence"
                        and not declaration["parametersJsonSchema"].get("required"),
                        "MCP schema was not returned")
                return self.call(tools, "mcp", "tool_call", {"name": MCP_TOOL, "arguments": {}})
            if self.rejected:
                outcome = self.result(results, "mcp").lower()
                require("reject" in outcome or "denied" in outcome or "cancel" in outcome,
                        "MCP result lacks a rejection")
                require(self.fixtures["mcp"] not in outcome, "rejected MCP returned the fixture value")
            else:
                self.result(results, "mcp", self.fixtures["mcp"])
        self.pending_action = None
        return {"role": "assistant", "content": "COEXISTENCE_DONE"}, "stop"

    def result(self, results, kind, expected=None):
        call_id = self.calls[kind]
        matches = [message["content"] for message in results if message.get("tool_call_id") == call_id]
        require(len(matches) == 1, f"missing or ambiguous {kind} result")
        content = result_text(matches[0])
        if expected is not None:
            require(expected in content, f"{kind} result lacks expected content")
        if kind != "search":
            self.completed[kind] = call_id
        return content

    def call(self, tools, kind, name, arguments):
        call_id = "call_" + secrets.token_hex(12)
        message, finish = self.tool_response(tools, name, arguments, call_id)
        if kind == "mcp":
            message["content"] = "Checking the selected read-only MCP tool."
        self.pending_action = (name, arguments)
        self.calls[kind] = call_id
        return message, finish

    @staticmethod
    def tool_response(tools, name, arguments, call_id):
        require(name in tools, f"{name} missing from native catalog")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= set(arguments) <= set(schema.get("properties", {})),
                f"{name} tool schema changed")
        return {"role": "assistant", "content": None, "tool_calls": [{
            "id": call_id, "type": "function",
            "function": {"name": name, "arguments": json.dumps(arguments)},
        }]}, "tool_calls"

    def search(self, tools):
        self.search_attempts += 1
        return self.call(tools, "search", "tool_search", {"query": "select:" + MCP_TOOL})

    def classifier_response(self, tools, messages):
        # Qwen uses a structured non-streaming side query for its permission
        # classifier. Match its pending action to the exact read-only call emitted
        # by this fixture. Any interactive ACP approval still has to occur.
        require(list(tools) == ["respond_in_schema"], "unexpected Qwen side query")
        content = messages[-1]["content"]
        if isinstance(content, list):
            content = "".join(item["text"] for item in content if item["type"] == "text")
        match = re.match(r"## Pending tool call to classify\n\nTool: ([^\n]+)\n"
                         r"Arguments:\n```json\n(.*?)\n```", content, re.S)
        require(match is not None, "permission classifier lacks a pending action")
        action = (match[1], json.loads(match[2]))
        require(action == self.pending_action or
                (action == (MCP_TOOL, {"server": "coexistence", "tool": "read_probe", "arguments": {}})
                 and self.pending_action ==
                 ("tool_call", {"name": MCP_TOOL, "arguments": {}})),
                "permission classifier received a foreign action")
        return self.tool_response(tools, "respond_in_schema", {"shouldBlock": False},
                                  "call_" + secrets.token_hex(12))

    def observe_terminal(self, events):
        if not self.rejected or "mcp" in self.completed:
            return
        # A host rejection can end the Qwen prompt immediately. Correlate the
        # native cancellation event to the emitted call when there is no continuation.
        call_id = self.calls.get("mcp")
        terminal = [event for event in events if call_id and event.get("type") == "chat.tool"
                    and event.get("call_id") == call_id and event.get("kind") == "complete"]
        if (len(terminal) == 1 and terminal[0].get("status") == "failed"
                and terminal[0]["payload"].get("error") == f'Tool "{MCP_TOOL}" was canceled by the user.'):
            self.completed["mcp"] = call_id

    def approves(self, metadata):
        return (self.pending_action == ("tool_call", {"name": MCP_TOOL, "arguments": {}})
                and "mcp" not in self.completed
                and metadata.get("title") == MCP_TOOL
                and metadata.get("arguments") == {})


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_POST(self):
        probe = self.server.probe
        code, content_type = 200, "application/json"
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated model request")
            require(self.path == "/v1/chat/completions", "unexpected Qwen model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid model request size")
            body = json.loads(self.rfile.read(size))
            message, finish = probe.response(body)
            value = {"id": "chatcmpl-" + secrets.token_hex(12), "object": "chat.completion", "model": body["model"],
                     "choices": [{"index": 0, "message": message, "finish_reason": finish}],
                     "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}}
            if body.get("stream"):
                if "tool_calls" in message:
                    message["tool_calls"][0]["index"] = 0
                value["object"] = "chat.completion.chunk"
                chunks = [
                    dict(value, choices=[{"index": 0, "delta": message, "finish_reason": None}],
                         usage=None),
                    dict(value, choices=[{"index": 0, "delta": {}, "finish_reason": finish}]),
                ]
                data = ("".join("data: " + json.dumps(chunk) + "\n\n" for chunk in chunks)
                        + "data: [DONE]\n\n").encode()
                content_type = "text/event-stream"
            else:
                data = json.dumps(value).encode()
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Chat Completions envelope")
            code, data = 422, b'{"error":"probe rejected model request"}'
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
