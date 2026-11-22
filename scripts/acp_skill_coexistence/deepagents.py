"""DeepAgents' native skills catalog, host file reader and stdio MCP probe."""
from contextlib import contextmanager
import errno
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
import sys
import threading

from verify_acp_integration import ProviderHandler, require, ROOT

from . import correlated_result

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
MODEL_ENDPOINT = API_PATH + "/chat/completions"
TRANSPORT = "stdio"
NATIVE_DIR = ".deepagents/skills"
MCP_TOOL = "coexistence__read_probe"
V4_ENTRY = "99235b0c39283be84bef07a2d3beb908533e93d5697e929eac62b450a01a7f52"


def launch(env):
    platform_target = (platform.system(), platform.machine())
    require(platform_target in {("Darwin", "arm64"), ("Linux", "x86_64"), ("Linux", "aarch64")},
            "unsupported DeepAgents coexistence test platform")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"
                           / "contracts.json").read_text())["deepagents"]
    require(contract["version"] == "0.1.42", "unexpected DeepAgents version")
    expected = contract["files"].copy()
    revision = 5
    if os.environ.get("ACP_DEEPAGENTS_V4_BASELINE") == "1":
        require(platform_target == ("Darwin", "arm64"), "v4 baseline requires Apple Silicon")
        revision = 4
        expected["dist/cli.js"] = V4_ENTRY
        expected.pop("dist/shiju-skills.mjs", None)
    directory = f"deepagents-0.1.42-model-source-v{revision}"
    installed = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = Path(os.environ.get("ACP_COEXISTENCE_DEEPAGENTS_RUNTIME",
                                 installed / "acp-runtime" / directory))
    require(source.is_dir(), "reviewed DeepAgents runtime unavailable")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / directory
    shutil.copytree(source, runtime, symlinks=True)
    package = runtime / "node_modules/deepagents-acp"
    files = {name: hashlib.sha256((package / name).read_bytes()).hexdigest()
             for name in expected}
    require(files == expected, "DeepAgents artifact digest mismatch")
    metadata = json.loads((package / "package.json").read_text())
    require((metadata["name"], metadata["version"]) == (contract["package"], contract["version"]),
            "DeepAgents package metadata identity mismatch")
    entry = runtime / "node_modules/.bin/deepagents-acp"
    require(entry.resolve() == package / contract["bin"], "DeepAgents executable differs from reviewed entry")
    return {"command": str(entry), "args": []}, {
        "version": contract["version"], "revision": revision, "integrity": contract["integrity"],
        "entry": contract["bin"], "model_endpoint": MODEL_ENDPOINT, "sha256": files,
    }


@contextmanager
def mcp_server(workspace, attachment, audit):
    # The reviewed stdio sandbox allows workspace reads and denies workspace
    # writes. Serve the attachment over a FIFO so the external writer can
    # independently count actual tool reads without changing that sandbox.
    fixtures = workspace / ".coexistence"
    fixtures.mkdir(mode=0o700)
    for name in ("acp_mount_integration.py", "verify_acp_integration.py"):
        shutil.copyfile(ROOT / "scripts" / name, fixtures / name)
    fifo = fixtures / "value"
    os.mkfifo(fifo, 0o600)
    value = attachment.read_bytes()
    stop = threading.Event()
    failures = []

    def serve():
        try:
            while not stop.is_set():
                try:
                    fd = os.open(fifo, os.O_WRONLY | os.O_NONBLOCK)
                except OSError as error:
                    if error.errno != errno.ENXIO:
                        raise
                    stop.wait(0.01)
                    continue
                with os.fdopen(fd, "wb") as writer:
                    replacement = fifo.with_suffix(".next")
                    os.mkfifo(replacement, 0o600)
                    replacement.replace(fifo)
                    with audit.open("a") as record:
                        record.write("stdio\n")
                    writer.write(value)
        except OSError as error:
            failures.append(type(error).__name__)

    executable = Path(sys.executable).resolve()
    # macOS framework launchers spawn the app executable, which the reviewed
    # sandbox disallows. Launch that same interpreter directly when available.
    app = Path(sys.base_prefix) / "Resources/Python.app/Contents/MacOS/Python"
    if app.is_file():
        executable = app.resolve()
    server = threading.Thread(target=serve)
    server.start()
    try:
        yield {"name": "coexistence", "command": str(executable),
               "args": [str(fixtures / "acp_mount_integration.py"), "--mcp-stdio",
                        str(fifo), "/dev/null"], "env": []}
    finally:
        stop.set()
        server.join()
        require(not failures, "DeepAgents stdio fixture audit failed")


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.failure = None
        self.pending = None
        self.reads = []

    def response(self, body):
        require(body.get("model") == MODEL, "unexpected DeepAgents upstream model")
        self.requests += 1
        messages = body["messages"]
        tools = {item["function"]["name"]: item["function"] for item in body["tools"]}
        require((MCP_TOOL in tools) == self.mounted, "unexpected DeepAgents MCP tool catalog")
        if self.requests == 1:
            catalog = "\n".join(str(item["content"]) for item in messages if item["role"] == "system")
            wire = json.dumps(body)
            legacy_baseline = not self.mounted and os.environ.get("ACP_DEEPAGENTS_V4_BASELINE") == "1"
            for kind, skill in self.fixtures["skills"].items():
                name = skill["name"]
                match = re.search(rf"^- \*\*{re.escape(name)}\*\*: [^\n]+\n"
                                  r"  → Read `([^`]+)` for full instructions$", catalog, re.M)
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require(bool(match) == (visible and not legacy_baseline), f"unexpected {kind} skill catalog")
                require(skill["body"] not in wire and skill["attachment"] not in wire,
                        "skill content appeared before activation")
                if visible:
                    path = Path(match[1]) if match else skill["path"] / "SKILL.md"
                    require(path.is_absolute() and path.resolve() == skill["path"].resolve() / "SKILL.md",
                            f"unexpected {kind} native catalog path")
                    if kind == "selected":
                        require(path.parent.is_symlink() and not path.is_symlink(),
                                "selected DeepAgents skill did not use the session view")
                    self.reads.extend(((kind, path, skill["body"]),
                                       (kind + "_read", path.parent / "references/value.txt",
                                        skill["attachment"])))
            require(self.fixtures["mcp"] not in wire, "MCP value appeared before invocation")

        if self.pending:
            step, _, _, identifier, expected = self.pending
            results = [dict(message, tool_use_id=message["tool_call_id"]) for message in messages
                       if message["role"] == "tool"]
            result = correlated_result(results, identifier, expected)
            if step == "mcp" and self.rejected:
                require(self.fixtures["mcp"] not in str(result["content"]),
                        "rejected DeepAgents MCP returned the fixture value")
            self.completed[step] = identifier
            self.pending = None

        if self.reads:
            step, path, value = self.reads.pop(0)
            return self.call(tools, step, "read_file", {"file_path": str(path), "offset": 0, "limit": 100}, value)
        if self.mounted and "mcp" not in self.completed:
            value = "Tool execution denied" if self.rejected else self.fixtures["mcp"]
            return self.call(tools, "mcp", MCP_TOOL, {}, value)
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def call(self, tools, step, name, args, expected):
        require(name in tools, f"{step} tool missing from DeepAgents catalog")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= set(args) <= set(schema.get("properties", {})),
                f"{step} tool schema changed")
        identifier = "call_" + secrets.token_hex(12)
        self.pending = step, name, args, identifier, expected
        return {"tool_calls": [{"index": 0, "id": identifier, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(args)}}]}, "tool_calls"

    def approves(self, metadata):
        return (self.pending is not None and self.pending[1] == MCP_TOOL
                and metadata.get("title") == f"Executing {MCP_TOOL}"
                and metadata.get("arguments") == {})

    def approves_skill(self, metadata):
        if self.pending is not None:
            step, name, args, _, _ = self.pending
            if (name == "read_file" and metadata.get("title") == "Reading file"
                    and metadata.get("arguments") == args):
                return step
        return None

    def expected_skill_approvals(self):
        return []  # The host ApproveReads policy covers read_file.


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.path == MODEL_ENDPOINT, "unexpected DeepAgents model endpoint")
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated DeepAgents fixture request")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid DeepAgents Chat Completions envelope")
            self.send_error(422, "probe rejected model request")
            return
        message = dict(delta, role="assistant")
        identifier = "chatcmpl-" + secrets.token_hex(12)
        self.send_response(200)
        if body.get("stream"):
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            for chunk, reason in ((message, None), ({}, finish)):
                frame = {"id": identifier, "object": "chat.completion.chunk", "created": 1,
                         "model": MODEL, "choices": [
                             {"index": 0, "delta": chunk, "finish_reason": reason}
                         ]}
                self.wfile.write(("data: " + json.dumps(frame) + "\n\n").encode())
            self.wfile.write(b"data: [DONE]\n\n")
        else:
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            for call in message.get("tool_calls", []):
                call.pop("index")
            self.wfile.write(json.dumps({"id": identifier, "object": "chat.completion",
                                         "created": 1, "model": MODEL, "choices": [
                                             {"index": 0, "message": message, "finish_reason": finish}
                                         ]}).encode())
