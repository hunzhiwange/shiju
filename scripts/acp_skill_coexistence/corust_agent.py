"""Exercise the Corust v0.6.0 native skill catalog and file reader."""

from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import runpy
import secrets
import shutil
import subprocess

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".corust-agent/skills"
VERSION = "0.6.0"
REVISION = f"corust-agent-{VERSION}-model-source-v4"
MCP_TOOL = "rustcodegraph_node"
MCP_ARGS = {"file": "fixture.rs", "name": "coexistence_fixture"}
NATIVE_TOOLS = {"read_file", "write_file", "glob", "grep"}


def launch(env):
    if platform.system() == "Linux":
        require(platform.machine() == "x86_64", "Corust 0.6.0 has no Linux ARM64 archive")
        archive = os.environ.get("ACP_CORUST_LINUX_ARCHIVE")
        require(archive, "set ACP_CORUST_LINUX_ARCHIVE to the fixed Corust Linux archive")
        installer = runpy.run_path(
            str(ROOT / "patches/acp-model-source/corust-agent/install_linux.py"))
        staging = (Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime"
                   / f"corust-agent-{VERSION}-linux-x64-native")
        staging.parent.mkdir(parents=True, exist_ok=True)
        entry, contract = installer["install_native"](Path(archive), staging)
        require(contract["version"] == VERSION, "unexpected Corust version")
        return {"command": str(entry), "args": []}, {
            "version": VERSION, "entry": entry.name,
            "native_sha256": contract["files"][entry.name],
        }
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Corust platform")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"
                           / "contracts.json").read_text())["corust-agent"]
    require(contract["version"] == VERSION, "unexpected Corust version")
    source = Path(os.environ.get("ACP_CORUST_REVIEWED_DIR",
                                 Path.home() / ".local/share/shiju/acp-runtime" / REVISION))
    patches = ROOT / "patches/acp-model-source/corust-agent"
    target = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    target.mkdir(parents=True, mode=0o700)
    for name, digest in contract["files"].items():
        artifact = source / name
        if name != "corust-agent-native":
            artifact = patches / ("launcher.py" if name == contract["bin"] else name)
        require(artifact.is_file(), f"missing reviewed Corust artifact: {name}")
        destination = target / name
        shutil.copyfile(artifact, destination)
        require(hashlib.sha256(destination.read_bytes()).hexdigest() == digest,
                f"reviewed Corust artifact digest mismatch: {name}")
        destination.chmod(0o700)
    return {"command": str(target / contract["bin"]), "args": []}, {
        "version": VERSION, "revision": REVISION, "sha256": contract["files"],
    }


@contextmanager
def mcp_server(workspace, attachment, _audit):
    graph = Path(os.environ.get("ACP_CORUST_GRAPH",
                                Path.home() / ".rustcodegraph/bin/rustcodegraph")).resolve(strict=True)
    require(graph.name == "rustcodegraph", "unexpected Corust graph executable")
    (workspace / "fixture.rs").write_text(
        f'pub fn coexistence_fixture() -> &\'static str {{ "{attachment.read_text()}" }}\n')
    private_home = workspace.parent / "home"
    indexed = subprocess.run(
        [str(graph), "init", "-i", str(workspace)], cwd=workspace, capture_output=True, timeout=60,
        env={"HOME": str(private_home), "RUSTCODEGRAPH_NO_DAEMON": "1",
             "TMPDIR": str(workspace.parent / "tmp"), "PATH": "/usr/bin:/bin"},
    )
    require(indexed.returncode == 0 and (workspace / ".rustcodegraph/rustcodegraph.db").is_file(),
            "Corust MCP fixture could not be indexed")
    yield {"name": "coexistence", "command": str(graph),
           "args": ["serve", "--mcp", "--path", str(workspace), "--no-watch"], "env": []}


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
        require(body.get("model") == MODEL and body.get("stream") is True,
                "unexpected Corust Chat Completions contract")
        self.requests += 1
        messages = body["messages"]
        tools = {row["function"]["name"]: row["function"] for row in body.get("tools", [])}
        require(NATIVE_TOOLS <= tools.keys(), "incomplete Corust native tool catalog")
        require((MCP_TOOL in tools) == self.mounted, "unexpected Corust MCP catalog")
        if self.requests == 1:
            wire = json.dumps(body)
            prompt = "\n".join(str(item["content"]) for item in messages if item["role"] == "system")
            for kind, skill in self.fixtures["skills"].items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in prompt) == visible, f"unexpected Corust {kind} skill catalog")
                require(skill["body"] not in wire and skill["attachment"] not in wire,
                        "Corust skill content appeared before activation")
            require(self.fixtures["mcp"] not in wire, "Corust graph value appeared before invocation")
            expected = ("native", "selected") if self.mounted else ("native",)
            for kind in expected:
                require(str(self.fixtures["skills"][kind]["path"] / "SKILL.md") in prompt,
                        f"Corust catalog omitted the {kind} skill path")
            return self.read(tools, "native", self.fixtures["skills"]["native"]["path"] / "SKILL.md")

        step, call_id, path = self.pending
        content = [message["content"] for message in messages if message.get("role") == "tool"
                   and message.get("tool_call_id") == call_id]
        require(len(content) == 1, f"missing correlated Corust {step} result")
        result = str(content[0])
        if step == "mcp":
            value = self.fixtures["mcp"]
            require(("MCP permission denied" in result and value not in result)
                    if self.rejected else value in result, "unexpected Corust MCP continuation")
        else:
            kind = step.removesuffix("_read")
            skill = self.fixtures["skills"][kind]
            expected = skill["body"] if step == kind else skill["attachment"]
            require(expected in result, f"missing correlated Corust {step} content")
        self.completed[step] = call_id
        if step in ("native", "selected"):
            return self.read(tools, step + "_read", path.parent / "references/value.txt")
        if self.mounted and step == "native_read":
            return self.read(tools, "selected", self.fixtures["skills"]["selected"]["path"] / "SKILL.md")
        if step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, MCP_ARGS)
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def read(self, tools, step, path):
        field, = tools["read_file"]["parameters"]["required"]
        return self.call(tools, step, "read_file", {field: str(path)}, path)

    def call(self, tools, step, name, arguments, path=None):
        schema = tools[name]["parameters"]
        require(set(schema.get("required", ())) <= arguments.keys() <= schema["properties"].keys(),
                f"unexpected Corust {step} schema")
        call_id = "call_" + secrets.token_hex(12)
        self.pending = (step, call_id, path)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == MCP_TOOL and metadata.get("arguments") == MCP_ARGS)

    def mcp_trace(self, events):
        # The bridge emits completed only after the Graph stdio request returns.
        # Its rejection branch posts the paired failure before dispatch.
        calls = [event for event in events if event.get("type") == "chat.tool"
                 and str(event.get("call_id", "")).startswith("corust-mcp-")]
        if not self.mounted:
            require(not calls, "unexpected Corust MCP events")
            return [], None
        start, end = calls if len(calls) == 2 else (None, None)
        require(start is not None and start.get("kind") == "start" and end.get("kind") == "complete"
                and start["call_id"] == end["call_id"], "unpaired Corust MCP call")
        require(start["payload"].get("title") == MCP_TOOL, "unexpected Corust MCP tool event")
        payload = end["payload"]
        require(payload.get("status") == ("error" if self.rejected else "completed"),
                "unexpected Corust MCP event status")
        output = json.dumps(payload)
        require(("MCP permission denied" in output and self.fixtures["mcp"] not in output)
                if self.rejected else self.fixtures["mcp"] in output, "unexpected Corust MCP result event")
        return ([] if self.rejected else [TRANSPORT]), end["call_id"]


    def approves_skill(self, metadata):
        if self.pending is None:
            return None
        step, _, path = self.pending
        if path is not None and metadata.get("title") == f"Read {path}":
            arguments = metadata.get("arguments")
            if isinstance(arguments, dict) and str(path) in arguments.values():
                return step

    def expected_skill_approvals(self):
        return []


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.path == "/v1/chat/completions", "unexpected Corust model endpoint")
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Corust model request")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Corust model request size")
            delta, finish = probe.response(json.loads(self.rfile.read(size)))
        except (AssertionError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Corust Chat Completions envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.chunk(MODEL, dict(delta, role="assistant"), None)
        self.chunk(MODEL, {}, finish)
        self.wfile.write(b"data: [DONE]\n\n")
