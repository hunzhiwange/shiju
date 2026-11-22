"""Hermes 0.19.0 native skill_view and stdio MCP through Chat Completions."""

import hashlib
from email.parser import BytesHeaderParser
from http.server import BaseHTTPRequestHandler
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess

from acp_skill_coexistence.mimo_code import mcp_server
from verify_acp_integration import require
from verify_acp_model_source import ROOT

MODEL = "hermes-coexistence"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".hermes/skills"
MCP_TOOL = "mcp__coexistence__read_probe"


def launch(env, prototype=False):
    linux = platform.system() == "Linux" and not prototype
    if linux:
        manifest = json.loads((ROOT / "patches/acp-model-source/hermes/linux.json").read_text())
        target = manifest["targets"].get(platform.machine())
        require(target and platform.libc_ver()[0] == "glibc", "unreviewed Hermes Linux target")
    else:
        require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
                "unreviewed Hermes platform")
    contracts = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(contracts.read_text())["hermes"]
    require(contract["version"] == "0.19.0", "Hermes version differs from coexistence pin")
    revision = "hermes-0.19.0-model-source-v2" if prototype else "hermes-0.19.0-model-source-v3"
    if prototype:
        metadata = ROOT / "docs/shiju/acp-model-source/evidence/hermes/patch.json"
        contract["files"] = {name: row["patched_sha256"]
                             for name, row in json.loads(metadata.read_text())["files"].items()
                             } | {"mcp/__init__.py": contract["files"]["mcp/__init__.py"]}
    config = Path(os.getenv("VIBEWINDOW_CONFIG_DIR") or Path.home() / ".local/share/shiju")
    supplied = os.getenv("HERMES_COEXISTENCE_RUNTIME")
    if linux:
        require(supplied, "set HERMES_COEXISTENCE_RUNTIME to the reviewed Linux fixture runtime")
    source = Path(supplied or config / "acp-runtime" / revision)
    require(source.is_dir(), "reviewed Hermes runtime is unavailable")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision
    shutil.copytree(source, runtime, symlinks=True)
    python_version = manifest["python"] if linux else "3.12"
    site = runtime / f"lib/python{python_version}/site-packages"
    for name, digest in contract["files"].items():
        require((site / name).is_file() and hashlib.sha256((site / name).read_bytes()).hexdigest() == digest,
                "Hermes artifact differs from contract: " + name)
    entry = runtime / "bin" / contract["bin"]
    if linux:
        require(entry.is_file() and entry.resolve() == entry and os.access(entry, os.X_OK),
                "Hermes Linux entry is missing, redirected, or non-executable")
    else:
        try:
            version = subprocess.run([str(entry), "acp", "--version"], env=env, capture_output=True,
                                     text=True, timeout=15)
        except subprocess.TimeoutExpired:
            raise AssertionError("Hermes version check timed out") from None
        require(version.returncode == 0 and re.search(r"\bv?0\.19\.0\b", version.stdout),
                "Hermes entry did not report the pinned version")
    artifact = {
        "version": contract["version"], "sha256": hashlib.sha256(entry.read_bytes()).hexdigest(),
        "contract_files": contract["files"], "wheel_sha256": contract["wheel_sha256"],
    }
    if linux:
        require(entry.relative_to(runtime).as_posix() == manifest["entry"]
                and contract["wheel_sha256"] == manifest["hermes"]["sha256"],
                "Linux and registry Hermes contracts differ")
        for name, expected in (("hermes_agent", manifest["hermes"]["version"]),
                               ("mcp", manifest["mcp"]["version"]),
                               ("agent_client_protocol", manifest["acp_sdk"]["version"])):
            [metadata] = site.glob(f"{name}-*.dist-info/METADATA")
            package = BytesHeaderParser().parsebytes(metadata.read_bytes())
            require(package["Name"].replace("-", "_") == name and package["Version"] == expected,
                    "Hermes installed package version mismatch: " + name)
        artifact.update(target=target, entry=manifest["entry"], mcp_version=manifest["mcp"]["version"],
                        model_endpoint="/v1/chat/completions")
    args = manifest["args"] if linux else ["acp"]
    return {"command": str(entry), "args": args, "env": {}}, artifact


class Probe:
    def __init__(self, fixtures, mounted, rejected, *, observe_stdio=True):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.calls = {}
        self.completed = {}
        self.failure = None
        self.audit = None
        self.observe_stdio = observe_stdio
        self.unmounted_call = None
        self.unmounted_rejected = False

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
            if self.mounted and self.observe_stdio:
                root = self.fixtures["skills"]["native"]["path"].parents[3]
                audits = list((root / "tmp").glob("shiju-mimo-mcp-*/mimo-fixture-audit"))
                require(len(audits) == 1, "missing or ambiguous Hermes stdio audit")
                self.audit = root / ("hermes-rejected-audit" if self.rejected else "hermes-mounted-audit")
                os.link(audits[0], self.audit)
                require(not self.audit.read_text(), "Hermes MCP ran before the model requested it")

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
                if kind == "native" and platform.system() == "Linux":
                    return self.call(tools, read, "read_file",
                                     {"path": str(skill["path"] / "references/value.txt")})
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
        elif platform.system() == "Linux":
            require(MCP_TOOL not in tools, "unmounted Hermes MCP tool reappeared")
            if self.unmounted_call is None:
                self.unmounted_call = "call_" + secrets.token_hex(12)
                return {"role": "assistant", "content": None, "tool_calls": [{
                    "id": self.unmounted_call, "type": "function",
                    "function": {"name": MCP_TOOL, "arguments": "{}"},
                }]}, "tool_calls"
            results = [message["content"] for message in messages
                       if message["role"] == "tool" and message["tool_call_id"] == self.unmounted_call]
            require(len(results) == 1 and isinstance(results[0], str)
                    and "does not exist" in results[0] and self.fixtures["mcp"] not in results[0],
                    "Hermes did not reject the unmounted MCP tool")
            self.unmounted_rejected = True
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

    def observe_terminal(self, _events):
        if platform.system() == "Linux" and not self.mounted and self.failure is None:
            require(self.unmounted_rejected, "missing continuation for the unmounted MCP call")
        return False

    def mcp_trace(self, _events):
        return self.audit.read_text().splitlines() if self.audit else [], self.completed.get("mcp")

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
