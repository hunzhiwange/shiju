"""Hermes 0.19.0 native skill_view and stdio MCP through Chat Completions."""

import hashlib
from http.server import BaseHTTPRequestHandler
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess

from verify_acp_integration import require
from verify_acp_model_source import ROOT

MODEL = "hermes-coexistence"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = "../home/.hermes/skills"
MCP_TOOL = "mcp__coexistence__read_probe"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Hermes platform")
    contracts = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(contracts.read_text())["hermes"]
    require(contract["version"] == "0.19.0", "Hermes version differs from coexistence pin")
    revision = "hermes-0.19.0-model-source-v2"
    config = Path(os.getenv("VIBEWINDOW_CONFIG_DIR") or Path.home() / ".local/share/shiju")
    source = Path(os.getenv("HERMES_COEXISTENCE_RUNTIME") or config / "acp-runtime" / revision)
    require(source.is_dir(), "reviewed Hermes runtime is unavailable")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision
    shutil.copytree(source, runtime, symlinks=True)
    site = runtime / "lib/python3.12/site-packages"
    for name, digest in contract["files"].items():
        require((site / name).is_file() and hashlib.sha256((site / name).read_bytes()).hexdigest() == digest,
                "Hermes artifact differs from contract: " + name)
    entry = runtime / "bin" / contract["bin"]
    try:
        version = subprocess.run([str(entry), "acp", "--version"], env=env, capture_output=True,
                                 text=True, timeout=15)
    except subprocess.TimeoutExpired:
        raise AssertionError("Hermes version check timed out") from None
    require(version.returncode == 0 and re.search(r"\bv?0\.19\.0\b", version.stdout),
            "Hermes entry did not report the pinned version")
    return {"command": str(entry), "args": ["acp"], "env": {}}, {
        "version": contract["version"], "sha256": hashlib.sha256(entry.read_bytes()).hexdigest(),
        "contract_files": contract["files"], "wheel_sha256": contract["wheel_sha256"],
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

    def response(self, body):
        require(body["model"] == MODEL, "unexpected upstream model")
        if not body.get("tools"):
            return {"role": "assistant", "content": "Ready"}, "stop"
        require(self.failure is None, self.failure)
        self.requests += 1
        messages = body["messages"]
        tools = {tool["function"]["name"]: tool["function"] for tool in body["tools"]}
        if self.requests == 1:
            prompt = "\n".join(str(message["content"]) for message in messages
                               if message["role"] == "system")
            catalog = "\n".join(re.findall(r"<available_skills>(.*?)</available_skills>", prompt, re.S))
            raw = json.dumps(body)
            for kind, skill in self.fixtures["skills"].items():
                expected = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog) == expected, f"unexpected {kind} skill catalog")
                require(skill["body"] not in raw and skill["attachment"] not in raw,
                        "skill content appeared before activation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected Hermes MCP declaration")
            require(self.fixtures["mcp"] not in raw, "MCP value appeared before invocation")

        for kind in ("native", "selected") if self.mounted else ("native",):
            skill = self.fixtures["skills"][kind]
            if kind not in self.calls:
                return self.call(tools, kind, "skill_view", {"name": skill["name"]})
            activation = json.loads(self.result(messages, kind, skill["body"]))
            require(activation["success"] is True, f"{kind} activation failed")
            actual = Path(activation["skill_dir"])
            require(actual.is_absolute() and actual.name == skill["name"], "invalid skill directory")
            if kind == "native":
                require(actual.resolve() == skill["path"].resolve(), "unexpected native skill source")
            else:
                require(actual != skill["path"] and
                        (actual / "SKILL.md").resolve() == (skill["path"] / "SKILL.md").resolve(),
                        "selected skill bypassed the session view")
            read = kind + "_read"
            if read not in self.calls:
                return self.call(tools, read, "skill_view",
                                 {"name": skill["name"], "file_path": "references/value.txt"})
            self.result(messages, read, skill["attachment"])
        if self.mounted:
            if "mcp" not in self.calls:
                return self.call(tools, "mcp", MCP_TOOL, {})
            result = self.result(messages, "mcp", None if self.rejected else self.fixtures["mcp"])
            if self.rejected:
                require(re.search(r"denied|reject|cancel", result, re.I) is not None
                        and self.fixtures["mcp"] not in result, "MCP result lacks a rejection")
        return {"role": "assistant", "content": "COEXISTENCE_DONE"}, "stop"

    def result(self, messages, kind, expected):
        call_id = self.calls[kind]
        results = [message["content"] for message in messages
                   if message["role"] == "tool" and message["tool_call_id"] == call_id]
        require(len(results) == 1 and isinstance(results[0], str), f"missing correlated {kind} result")
        require(expected is None or expected in results[0], f"{kind} result lacks fixture content")
        self.completed[kind] = call_id
        return results[0]

    def call(self, tools, kind, name, arguments):
        require(name in tools, f"{name} missing from Hermes tool catalog")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= set(arguments) <= set(schema.get("properties", {})),
                f"{name} schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.calls[kind] = call_id
        return {"role": "assistant", "content": None, "tool_calls": [{
            "id": call_id, "type": "function",
            "function": {"name": name, "arguments": json.dumps(arguments)},
        }]}, "tool_calls"

    def approves(self, metadata):
        return ("mcp" in self.calls and "mcp" not in self.completed
                and metadata.get("title") == MCP_TOOL and metadata.get("arguments") == {})


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_POST(self):
        probe = self.server.probe
        code, content_type = 200, "application/json"
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Hermes model request")
            require(self.path == "/v1/chat/completions", "unexpected Hermes model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid model request size")
            body = json.loads(self.rfile.read(size))
            message, finish = probe.response(body)
            value = {"id": "chatcmpl-" + secrets.token_hex(12), "object": "chat.completion",
                     "model": body["model"], "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
                     "choices": [{"index": 0, "message": message, "finish_reason": finish}]}
            if body.get("stream"):
                if "tool_calls" in message:
                    message["tool_calls"][0]["index"] = 0
                value["object"] = "chat.completion.chunk"
                chunks = [
                    dict(value, choices=[{"index": 0, "delta": message, "finish_reason": None}], usage=None),
                    dict(value, choices=[{"index": 0, "delta": {}, "finish_reason": finish}]),
                ]
                data = ("".join("data: " + json.dumps(chunk) + "\n\n" for chunk in chunks)
                        + "data: [DONE]\n\n").encode()
                content_type = "text/event-stream"
            else:
                data = json.dumps(value).encode()
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Hermes Chat Completions envelope")
            code, data = 422, b'{"error":"probe rejected model request"}'
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
