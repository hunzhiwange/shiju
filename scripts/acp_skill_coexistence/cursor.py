"""Probe the pinned Cursor ACP with native skill reads and read-only stdio MCP."""

import hashlib
import json
from pathlib import Path
import platform
import secrets
import shutil
import subprocess
import sys
import tempfile

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".cursor/skills"
VERSION = "2026.11.11-b912975"
MANAGED_DIR = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"
PATCH_DIR = ROOT / "patches/acp-model-source/cursor"


def launch(env):
    if platform.system() == "Linux":
        return launch_linux(env)
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Cursor platform")
    contract = json.loads((MANAGED_DIR / "contracts.json").read_text())["cursor"]
    require(contract["version"] == VERSION, "unexpected Cursor artifact version")
    target = Path(env["HOME"]) / ".local/share/shiju/acp-runtime" / f"cursor-{VERSION}-model-source-v6"
    target.parent.mkdir(parents=True)
    with tempfile.TemporaryDirectory(dir=env["TMPDIR"], prefix="cursor-") as directory:
        original = Path.home() / ".local/share/cursor-agent/versions" / VERSION
        if original.is_dir():
            dist = target / "dist-package"
            for name in contract["files"]:
                relative = Path(name).relative_to("dist-package")
                if relative == Path("session-model.cjs"):
                    continue
                destination = dist / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(original / relative, destination)
            command = [sys.executable, str(PATCH_DIR / "apply.py"), str(dist)]
        else:
            assets = Path(directory)
            shutil.copy2(MANAGED_DIR / "contracts.json", assets)
            shutil.copytree(PATCH_DIR, assets / "cursor")
            command = [sys.executable, str(MANAGED_DIR / "install.py"), str(assets), str(target), "cursor"]
        try:
            result = subprocess.run(command, env=env, capture_output=True, timeout=300)
        except subprocess.TimeoutExpired as error:
            raise AssertionError("reviewed Cursor runtime installation timed out") from error
        require(result.returncode == 0, "reviewed Cursor runtime installation failed")
    hashes = {name: hashlib.sha256((target / name).read_bytes()).hexdigest()
              for name in contract["files"]}
    require(hashes == contract["files"], "Cursor managed runtime digest mismatch")
    command = target / contract["bin"]
    try:
        version = subprocess.run([str(command), "--version"], env=env, cwd=target,
                                 capture_output=True, timeout=15)
    except subprocess.TimeoutExpired as error:
        raise AssertionError("Cursor version check timed out") from error
    require(version.returncode == 0 and version.stdout.strip() == VERSION.encode(),
            "unexpected Cursor executable version")
    return {"command": str(command), "args": ["acp"]}, {
        "revision": target.name, "version": VERSION, "archive_sha256": contract["archive_sha256"],
        "file_count": len(hashes), "entry_sha256": hashes[contract["bin"]],
        "model_bridge_sha256": hashes["dist-package/session-model.cjs"],
    }


def launch_linux(env):
    manifest = json.loads((PATCH_DIR / "linux-artifacts.json").read_text())
    contract = manifest["targets"].get(platform.machine())
    require(contract is not None and contract["version"] == manifest["version"] == VERSION,
            "unreviewed Cursor Linux target")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime"
    runtime.mkdir(parents=True)
    target = runtime / f"cursor-{VERSION}-model-source-v6"
    with tempfile.TemporaryDirectory(dir=env["TMPDIR"], prefix="cursor-") as directory:
        assets = Path(directory)
        shutil.copy2(MANAGED_DIR / "contracts.json", assets)
        shutil.copytree(PATCH_DIR, assets / "cursor")
        command = [sys.executable, str(MANAGED_DIR / "install.py"), str(assets), str(target), "cursor"]
        try:
            result = subprocess.run(command, env=env, capture_output=True, timeout=300)
        except subprocess.TimeoutExpired as error:
            raise AssertionError("reviewed Linux Cursor runtime installation timed out") from error
        require(result.returncode == 0, "reviewed Linux Cursor runtime installation failed")
    entry = target / contract["bin"]
    digest = hashlib.sha256(entry.read_bytes()).hexdigest()
    require(digest == contract["files"][contract["bin"]], "Cursor Linux entry digest mismatch")
    node_digest = hashlib.sha256()
    with (entry.parent / "node").open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            node_digest.update(block)
    require(node_digest.hexdigest() == contract["files"]["dist-package/node"],
            "Cursor Linux bundled Node digest mismatch")
    try:
        version = subprocess.run([str(entry), "--version"], env=env, cwd=target,
                                 capture_output=True, timeout=15)
    except subprocess.TimeoutExpired as error:
        raise AssertionError("Cursor Linux version check timed out") from error
    require(version.returncode == 0 and version.stdout.strip() == VERSION.encode(),
            "unexpected Linux Cursor executable version")
    return {"command": str(entry), "args": ["acp"]}, {
        "entry": contract["bin"], "version": VERSION, "entry_sha256": digest,
        "archive_sha256": contract["archive_sha256"], "node_sha256": node_digest.hexdigest(),
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
        self.locations = {}
        self.mcp_tool = None

    def response(self, body):
        require(body["model"] == MODEL and body["stream"] is True,
                "unexpected Cursor Chat Completions contract")
        self.requests += 1
        tools = {item["function"]["name"]: item["function"] for item in body["tools"]}
        if self.requests == 1:
            require("shell" in tools, "Cursor native shell absent")
            directory = next(message["content"] for message in body["messages"]
                             if message["role"] == "system")
            context = json.loads(directory.split("Workspace and rules:\n", 1)[1])
            catalog = context.get("agentSkills", [])
            envelope = json.dumps(body)
            for kind, skill in self.fixtures["skills"].items():
                matches = [row for row in catalog
                           if Path(row.get("fullPath", "")).parent.name == skill["name"]]
                expected = kind == "native" or (kind == "selected" and self.mounted)
                require(len(matches) == int(expected), f"unexpected Cursor {kind} skill catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Cursor disclosed fixture contents before activation")
                if matches:
                    path = Path(matches[0]["fullPath"])
                    require(path.is_absolute() and path.name == "SKILL.md", "invalid Cursor skill location")
                    if kind == "native":
                        require(path.resolve() == (skill["path"] / "SKILL.md").resolve(),
                                "Cursor native skill location changed")
                    else:
                        require(any(part.startswith("shiju-session-skills-") for part in path.parts),
                                "Cursor selected skill bypassed its session view")
                    self.locations[kind] = path
            require(self.fixtures["mcp"] not in envelope, "Cursor disclosed MCP fixture before invocation")
            mcp_tools = [name for name in tools if name == "coexistence-read_probe"]
            require(len(mcp_tools) == int(self.mounted), "unexpected Cursor MCP directory")
            self.mcp_tool = mcp_tools[0] if mcp_tools else None
            return self.call(tools, "native", "read", {"path": str(self.locations["native"])})

        step, call_id = self.pending
        matches = [message for message in body["messages"]
                   if message["role"] == "tool" and message["tool_call_id"] == call_id]
        require(len(matches) == 1, "missing or ambiguous Cursor native tool result")
        output = json.loads(matches[0]["content"])
        if step == "mcp":
            value = self.fixtures["mcp"]
            if self.rejected:
                require("rejected" in output and value not in json.dumps(output),
                        "Cursor MCP denial did not reach model")
            else:
                require("success" in output and value in json.dumps(output["success"]),
                        "missing Cursor MCP tool value")
        else:
            kind = step.removesuffix("_read")
            field = "attachment" if step.endswith("_read") else "body"
            require(self.fixtures["skills"][kind][field] in output.get("success", {}).get("content", ""),
                    f"missing Cursor {step} content")
        self.completed[step] = call_id
        if step in self.locations:
            path = self.locations[step].parent / "references/value.txt"
            return self.call(tools, step + "_read", "read", {"path": str(path)})
        if self.mounted and step == "native_read":
            return self.call(tools, "selected", "read", {"path": str(self.locations["selected"])})
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", self.mcp_tool, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def call(self, tools, step, name, args):
        require(name in tools, f"Cursor omitted tool for {step}")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= args.keys() <= schema.get("properties", {}).keys(),
                f"unexpected Cursor tool schema for {step}")
        call_id = "cursor_probe_" + secrets.token_hex(12)
        self.pending = (step, call_id)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(args)}}]}, "tool_calls"

    def approves(self, metadata):
        expected_args = {} if platform.system() == "Linux" else None
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == f"{self.mcp_tool}: read_probe"
                and metadata.get("arguments") == expected_args)


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.path == "/v1/chat/completions", "unexpected Cursor model protocol")
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Cursor model request")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Cursor model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, TypeError, ValueError, StopIteration) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Cursor model envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.chunk(body["model"], dict(delta, role="assistant"), None)
        self.chunk(body["model"], {}, finish)
        self.wfile.write(b"data: [DONE]\n\n")
