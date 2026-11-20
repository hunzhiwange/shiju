"""Probe the Poolside 1.0.16 managed Docker skill boundary."""
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess

from verify_acp_integration import require
from verify_acp_model_source import ROOT

from . import correlated_result
from .opencode import Handler, Probe as ChatProbe

VERSION = "1.0.16"
REVISION = f"poolside-{VERSION}-model-source-v2"
MODEL = "selected-model"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".agents/skills"
# Keep the isolated HOME's Docker socket path within Darwin's Unix socket limit.
TEMP_PARENT = "/tmp"
MCP_TOOL = "coexistence__read_probe"


def docker(*args):
    try:
        return subprocess.run(["docker", *args], capture_output=True, text=True, timeout=20)
    except subprocess.TimeoutExpired:
        raise AssertionError("Poolside Docker command timed out") from None


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Poolside platform")
    require(docker("info", "--format", "{{.ServerVersion}}").returncode == 0,
            "Poolside requires a running Docker engine")
    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(manifest.read_text())["poolside"]
    require(contract["version"] == VERSION, "unexpected Poolside contract version")
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = Path(os.environ.get("ACP_POOLSIDE_REVIEWED_DIR", base / "acp-runtime" / REVISION))
    require(source.is_dir(), "reviewed Poolside runtime unavailable")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    runtime.mkdir(parents=True, mode=0o700)
    for name, expected in contract["files"].items():
        entry = runtime / name
        shutil.copy2(source / name, entry)
        require(hashlib.sha256(entry.read_bytes()).hexdigest() == expected,
                "Poolside artifact digest mismatch")
    command = runtime / contract["bin"]
    try:
        version = subprocess.run([str(command), "--version"], env=env, capture_output=True, timeout=15)
    except subprocess.TimeoutExpired:
        raise AssertionError("Poolside version check timed out") from None
    require(version.returncode == 0 and version.stdout.strip() == VERSION.encode(),
            "unexpected Poolside executable version")

    # The reviewed launcher consults HOME for the local Docker socket. Give the
    # isolated gateway only that host socket path, leaving its other settings private.
    socket = Path.home() / ".docker/run/docker.sock"
    if socket.is_socket():
        link = Path(env["HOME"]) / ".docker/run/docker.sock"
        link.parent.mkdir(parents=True, mode=0o700)
        link.symlink_to(socket)
    else:
        require(Path("/var/run/docker.sock").is_socket(), "reviewed Poolside Docker socket unavailable")
    return {"command": str(command), "args": ["acp"]}, {
        "version": VERSION, "platform": "darwin-arm64", "revision": REVISION,
        "sha256": contract["files"][contract["bin"]], "files": contract["files"],
    }


@contextmanager
def mcp_server(workspace, attachment, audit):
    from verify_acp_skill_coexistence import mcp_server as private_mcp

    with private_mcp(TRANSPORT, attachment, audit) as server:
        try:
            yield server
        finally:
            # The gateway has exited. Remove only sandboxes identified by the
            # exact bind source of this probe's private workspace.
            containers = docker("ps", "-aq")
            require(containers.returncode == 0, "could not list Poolside probe containers")
            for container in containers.stdout.splitlines():
                details = docker("inspect", "--format", "{{json .Mounts}}", container)
                if details.returncode == 0 and any(
                    Path(mount["Source"]).resolve() == workspace
                    for mount in json.loads(details.stdout)
                ):
                    require(docker("rm", "-f", container).returncode == 0,
                            "could not remove a Poolside probe container")


class Probe(ChatProbe):
    def response(self, body):
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if not tools:
            return {"content": "Poolside fixture session"}, "stop"
        require(body["model"] == MODEL, "unexpected Poolside model")
        self.requests += 1
        native = self.fixtures["skills"]["native"]
        if self.requests == 1:
            envelope = json.dumps(body)
            for skill in self.fixtures["skills"].values():
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Poolside exposed skill content before activation")
            require(self.fixtures["mcp"] not in envelope, "Poolside exposed MCP data before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected Poolside MCP catalog")
            require("skill" not in tools, "Poolside now publishes a skill tool and needs a new review")
            require(native["name"] not in envelope, "Poolside now publishes native skill metadata")
            # Establish a real read baseline while keeping it separate from
            # skill activation. The path comes from the private fixture.
            return self.call(tools, "ordinary_read", "read",
                             {"path": str(native["path"] / "references/value.txt")})
        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"])
                   for row in body["messages"] if row["role"] == "tool"]
        correlated_result(results, call_id, native["attachment"])
        self.completed[step] = call_id
        self.failure = "Poolside managed model request has no native skill catalog or activation tool"
        self.pending = None
        return {"content": "POOLSIDE_READ_BASELINE_DONE"}, "stop"

    def observe_terminal(self, events):
        if self.mounted and any(
            row.get("type") == "chat.error"
            and "poolside: no verified skill directory mount branch" in str(row.get("error", ""))
            for row in events
        ):
            self.failure = "Poolside host rejected the unverified skill mount before model inference"
        return False

    def approves(self, _metadata):
        return False
