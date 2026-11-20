"""Mistral Vibe's native Chat Completions, skill and stdio MCP protocol."""

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

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
# The gateway fixture has its own HOME. VIBE_HOME stays bound to the model profile.
NATIVE_DIR = "../home/.agents/skills"
DISABLED_SKILL = "disabled-vibe-coexistence-probe"
REVISION = "vibe-3.0.4-model-source-v4"
MCP_FUNCTION = "mcp_coexistence.read_probe"
PROGRAM = "run_typescript"
SEARCH = "search_tool_functions"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Vibe platform")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json")
                          .read_text())["vibe"]
    require(contract["version"] == "3.0.4", "Vibe version differs from coexistence pin")
    config = Path(env["VIBEWINDOW_CONFIG_DIR"])
    private = config / "acp-runtime" / REVISION
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    runtime = Path(os.environ.get("VIBE_COEXISTENCE_RUNTIME", base / "acp-runtime" / REVISION))
    require(runtime.is_dir(), "reviewed Vibe runtime is unavailable")
    shutil.copytree(runtime, private, symlinks=True)
    site = private / "lib/python3.12/site-packages"
    for name, digest in contract["files"].items():
        require(hashlib.sha256((site / name).read_bytes()).hexdigest() == digest,
                "Vibe artifact digest differs from contract")
    entry = private / "bin/vibe-acp"
    env["VIBE_TEST_DISABLE_KEYRING"] = "1"
    env["PYTHON_KEYRING_BACKEND"] = "keyring.backends.null.Keyring"
    version = subprocess.run([str(entry), "--version"], env=env, cwd=config,
                             capture_output=True, timeout=15, check=False)
    require(version.returncode == 0 and b"3.0.4" in version.stdout,
            "Vibe entry did not report the pinned version")
    return {"command": str(entry), "args": [], "env": {
        "VIBE_TEST_DISABLE_KEYRING": "1",
        "PYTHON_KEYRING_BACKEND": "keyring.backends.null.Keyring",
    }}, {"version": contract["version"], "revision": REVISION, "sha256": contract["files"],
         "entry_sha256": hashlib.sha256(entry.read_bytes()).hexdigest(),
         "wheel_sha256": contract["wheel_sha256"]}


def prepare_workspace(workspace):
    skill = workspace / NATIVE_DIR / DISABLED_SKILL
    skill.mkdir()
    (skill / "SKILL.md").write_text(
        f"---\nname: {DISABLED_SKILL}\ndescription: Explicit invocation only\n"
        "disable-model-invocation: true\n---\nDisabled fixture body.\n")


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
        require(body["model"] == MODEL, "unexpected Vibe model")
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if not tools:
            return {"content": "Fixture resources"}, "stop"
        messages = body["messages"]
        self.requests += 1
        if self.requests == 1:
            request = json.dumps(body)
            require(SEARCH in tools and PROGRAM in tools, "Vibe programmatic discovery missing")
            catalog = set(re.findall(r"<skill>\s*<name>([^<]+)</name>", "\n".join(
                item["content"] for item in messages if item.get("role") == "system")))
            require(DISABLED_SKILL not in catalog, "Vibe exposed the explicit-only native skill")
            for kind, skill in self.fixtures["skills"].items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog) == visible, f"unexpected Vibe {kind} skill catalogue")
                require(skill["body"] not in request and skill["attachment"] not in request,
                        "Vibe skill content leaked before activation")
            require(self.fixtures["mcp"] not in request, "MCP value leaked before invocation")
        for kind in ("native", "selected") if self.mounted else ("native",):
            skill = self.fixtures["skills"][kind]
            if kind not in self.calls:
                return self.call(tools, kind, "skill", {"name": skill["name"]})
            activation = self.result(messages, kind, skill["body"])
            read = kind + "_read"
            if read not in self.calls:
                match = re.search(r"Base directory for this skill: ([^\n]+)", activation)
                require(match is not None, "Vibe activation omitted the base directory")
                base = Path(match[1].strip())
                require(base.is_absolute() and base.name == skill["name"] and
                        (base / "SKILL.md").resolve() == (skill["path"] / "SKILL.md").resolve(),
                        "Vibe activated an unexpected skill path")
                if kind == "native":
                    require(base == skill["path"].resolve(), "Vibe bypassed the isolated native user directory")
                else:
                    require(base.parent.parent.name.startswith("shiju-session-skills-"),
                            "Vibe bypassed the mounted view")
                schema = tools["read_file"]["parameters"]["properties"]
                field = "path" if "path" in schema else "file_path"
                return self.call(tools, read, "read_file", {field: str(base / "references/value.txt")})
            self.result(messages, read, skill["attachment"])

        if "search" not in self.calls:
            return self.call(tools, "search", SEARCH, {"mode": "details", "functions": [MCP_FUNCTION]})
        search = self.result(messages, "search", record=False)
        require(re.search(rf"^loadedFunctionCount: {int(self.mounted)}$", search, re.M),
                "unexpected Vibe MCP discovery count")
        require((f"function: {MCP_FUNCTION}" in search) == self.mounted,
                "unexpected Vibe MCP discovery result")
        if self.mounted:
            if "mcp" not in self.calls:
                return self.call(tools, "mcp", PROGRAM, {
                    "code": self.mcp_code(),
                })
            result = self.result(messages, "mcp", None if self.rejected else self.fixtures["mcp"])
            if self.rejected:
                require(re.search(r"reject|denied|cancel", result, re.I) and self.fixtures["mcp"] not in result,
                        "Vibe MCP result lacks a refusal")
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def call(self, tools, kind, name, arguments):
        require(name in tools, f"Vibe {kind} tool missing")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= set(arguments) <= set(schema.get("properties", {})),
                f"Vibe {kind} schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.calls[kind] = call_id
        self.pending = (kind, arguments)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def result(self, messages, kind, expected=None, *, record=True):
        call_id = self.calls[kind]
        results = [item["content"] for item in messages if item.get("role") == "tool"
                   and item.get("tool_call_id") == call_id]
        require(len(results) == 1 and isinstance(results[0], str), f"missing Vibe {kind} result")
        content = results[0]
        if content.startswith("{"):
            decoded = json.loads(content)
            if isinstance(decoded, dict) and isinstance(decoded.get("content"), str):
                content = decoded["content"]
        require(expected is None or expected in content, f"Vibe {kind} result lacks expected content")
        if record:
            self.completed[kind] = call_id
        return content

    def approves(self, metadata):
        return (self.pending == ("mcp", {"code": self.mcp_code()})
                and metadata.get("title") == MCP_FUNCTION and metadata.get("arguments") == {})

    def approves_skill(self, metadata):
        if self.pending is None or self.pending[0] != "native_read":
            return None
        _, args = self.pending
        raw = metadata.get("arguments")
        if (metadata.get("title") == "file_system.read_file" and isinstance(raw, dict)
                and raw.get("filePath") == (args.get("path") or args.get("file_path"))):
            return "native_read"
        return None

    def expected_skill_approvals(self):
        # Native user files are outside the workspace. The projected attachment
        # resolves inside it, where the supplier allows reads.
        return ["native_read"]

    @staticmethod
    def mcp_code():
        return f"async function main() {{ return await tools.{MCP_FUNCTION}({{}}); }}"


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Vibe model request")
            require(self.path == "/v1/chat/completions", "unexpected Vibe model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Vibe request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Vibe Chat Completions envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        if body.get("stream"):
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            self.chunk(body["model"], dict(delta, role="assistant"), None)
            self.chunk(body["model"], {}, finish)
            self.wfile.write(b"data: [DONE]\n\n")
        else:
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"id": "chatcmpl-fixture", "object": "chat.completion",
                "model": body["model"], "choices": [{"index": 0, "finish_reason": finish,
                "message": dict(delta, role="assistant")}]}).encode())
