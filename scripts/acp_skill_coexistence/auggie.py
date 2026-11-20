"""Exercise Auggie 0.37.1 native skill reads and stdio MCP via Chat Completions."""

import hashlib
import json
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess
import sys
import tempfile

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

from . import correlated_result

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".augment/skills"
REVISION = "auggie-0.37.1-model-source-v1"
MCP_TOOL = "read_probe_coexistence"
MANAGED_DIR = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"
PATCH_DIR = ROOT / "patches/acp-model-source/auggie"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Auggie platform")
    manifest = MANAGED_DIR / "contracts.json"
    contract = json.loads(manifest.read_text())["auggie"]
    require(contract["version"] == "0.37.1", "unexpected Auggie contract version")
    target = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    target.parent.mkdir()
    with tempfile.TemporaryDirectory() as directory:
        assets = Path(directory)
        shutil.copy2(manifest, assets)
        shutil.copytree(PATCH_DIR, assets / "auggie")
        try:
            installed = subprocess.run(
                [sys.executable, str(MANAGED_DIR / "install.py"), str(assets), str(target), "auggie"],
                env=env, capture_output=True, timeout=180)
        except subprocess.TimeoutExpired as error:
            raise AssertionError("Auggie reviewed runtime installation timed out") from error
        require(installed.returncode == 0, "Auggie reviewed runtime installation failed")
    package = target / "node_modules" / contract["package"]
    files = {name: hashlib.sha256((package / name).read_bytes()).hexdigest()
             for name in contract["files"]}
    require(files == contract["files"], "Auggie runtime digest mismatch")
    entry = target / "node_modules/.bin/auggie"
    require(entry.resolve() == (package / contract["bin"]).resolve(),
            "unexpected Auggie executable target")
    try:
        version = subprocess.run([str(entry), "--version"], env=env,
                                 capture_output=True, timeout=15)
    except subprocess.TimeoutExpired as error:
        raise AssertionError("Auggie version check timed out") from error
    require(version.returncode == 0 and version.stdout.strip() == b"0.37.1 (commit e8d38687)",
            "unexpected Auggie executable version")
    return {"command": str(entry), "args": ["--acp"]}, {
        "version": contract["version"], "revision": REVISION, "sha256": files,
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
        self.paths = {}

    def response(self, body):
        require(body["model"] == MODEL, "unexpected Auggie model")
        self.requests += 1
        tools = {row["function"]["name"]: row["function"] for row in body.get("tools", [])}
        messages = body["messages"]
        skills = self.fixtures["skills"]
        require((MCP_TOOL in tools) == self.mounted, "unexpected Auggie stdio MCP catalogue")
        if self.mounted:
            schema = tools[MCP_TOOL]["parameters"]
            require(schema["type"] == "object" and schema["properties"] == {}
                    and not schema.get("required"), "unexpected Auggie read-only MCP schema")
        if self.requests == 1:
            self.check_catalogue(body)
            return self.read(tools, "native", self.paths["native"])
        step, call_id = self.pending
        results = [{"tool_use_id": row["tool_call_id"], "content": row["content"]}
                   for row in messages if row["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and re.search(r"reject|denied|cancel", result, re.I))
                    if self.rejected else value in result, "unexpected Auggie MCP continuation")
        else:
            kind = step.removesuffix("_read")
            marker = "attachment" if step.endswith("_read") else "body"
            require(skills[kind][marker] in result, f"Auggie {step} did not return its {marker}")
        self.completed[step] = call_id
        if step in ("native", "selected"):
            return self.read(tools, step + "_read", self.paths[step].parent / "references/value.txt")
        if self.mounted and step == "native_read":
            return self.read(tools, "selected", self.paths["selected"])
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def check_catalogue(self, body):
        entries = [json.loads(line) for row in body["messages"] if row["role"] == "system"
                   for line in row["content"].splitlines() if line.startswith('{"name":')
                   and '"location":' in line]
        request = json.dumps(body)
        for kind, skill in self.fixtures["skills"].items():
            visible = kind == "native" or (kind == "selected" and self.mounted)
            matches = [row for row in entries if row["name"] == skill["name"]]
            require(len(matches) == int(visible), f"unexpected Auggie {kind} skill catalogue")
            require(skill["body"] not in request and skill["attachment"] not in request,
                    "Auggie skill content appeared before activation")
            if matches:
                path = Path(matches[0]["location"])
                require(path.is_absolute() and path.name == "SKILL.md"
                        and path.parent.name == skill["name"],
                        "unexpected Auggie skill path")
                if kind == "native":
                    require(path == skill["path"] / "SKILL.md", "Auggie bypassed the native directory")
                else:
                    require(any(part.startswith("shiju-session-skills-") for part in path.parts),
                            "Auggie bypassed the session skill view")
                self.paths[kind] = path
        require(self.fixtures["mcp"] not in request, "Auggie MCP content appeared before invocation")

    def read(self, tools, step, path):
        return self.call(tools, step, "view", {"type": "file", "path": str(path)})

    def call(self, tools, step, name, arguments):
        require(name in tools, f"Auggie {step} tool missing")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= arguments.keys() <= schema["properties"].keys(),
                f"Auggie {step} tool schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.pending = (step, call_id)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == MCP_TOOL and metadata.get("arguments") == {})


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Auggie model request")
            require(self.path == "/v1/chat/completions", "unexpected Auggie model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Auggie model request size")
            body = json.loads(self.rfile.read(size))
            require(body.get("stream") is True, "Auggie did not request model streaming")
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Auggie Chat Completions envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.chunk(body["model"], dict(delta, role="assistant"), None)
        self.chunk(body["model"], {}, finish)
        self.wfile.write(b"data: [DONE]\n\n")
