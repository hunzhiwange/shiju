"""Verify Poolside skills and approved MCP through the managed Docker entry."""
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess

from verify_acp_integration import require
from verify_acp_model_source import ROOT

from . import correlated_result
from .cortex_code import mcp_server as sandboxed_stdio
from .opencode import Handler, Probe as ChatProbe

VERSION = "1.0.16"
REVISION = f"poolside-{VERSION}-model-source-v3"
MODEL = "selected-model"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".agents/skills"
# Keep the isolated HOME's Docker socket path within Darwin's Unix socket limit.
TEMP_PARENT = "/tmp"
MCP_TOOL = "coexistence__read_probe"
LINUX_CONTRACT = ROOT / "patches/acp-model-source/poolside/linux.json"


def linux_docker_socket():
    socket = Path(os.environ.get("ACP_POOLSIDE_DOCKER_SOCKET", ""))
    require(socket.is_absolute() and socket.is_socket(),
            "set ACP_POOLSIDE_DOCKER_SOCKET to the dedicated fixture daemon's Unix socket")
    return socket.resolve()


def docker(*args):
    endpoint = ["--host", f"unix://{linux_docker_socket()}"] if platform.system() == "Linux" else []
    try:
        return subprocess.run(["docker", *endpoint, *args], capture_output=True, text=True, timeout=20)
    except subprocess.TimeoutExpired:
        raise AssertionError("Poolside Docker command timed out") from None


def launch(env):
    if platform.system() == "Linux":
        return launch_linux(env)
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


def launch_linux(env):
    machine = platform.machine()
    release = json.loads(LINUX_CONTRACT.read_text())
    artifact = release["artifacts"].get(f"{machine}-unknown-linux-gnu")
    require(artifact is not None and release["version"] == VERSION, "unreviewed Linux Poolside target")
    require(os.geteuid() != 0, "run the Poolside gateway fixture as a non-root user")
    socket = linux_docker_socket()
    info = docker("info", "--format", "{{.OSType}}/{{.Architecture}}")
    require(info.returncode == 0 and info.stdout.strip() in {f"linux/{machine}", artifact["docker_platform"]},
            "Poolside requires an isolated Docker daemon of the target architecture")

    configured = os.environ.get("ACP_POOLSIDE_REVIEWED_DIR")
    require(configured and Path(configured).is_absolute(),
            "set ACP_POOLSIDE_REVIEWED_DIR to the private Linux managed install")
    source = Path(configured).resolve(strict=True)
    require(source.name == release["runtime_dir"], "unexpected Poolside Linux managed installation")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / source.name
    runtime.parent.mkdir(mode=0o700)
    shutil.copytree(source, runtime, symlinks=True)
    native = runtime / "pool-native"
    require(native.is_file() and not native.is_symlink(), "Poolside native binary missing")
    native_digest = hashlib.sha256(native.read_bytes()).hexdigest()
    require(native_digest == artifact["native_sha256"], "Poolside Linux native digest mismatch")
    command = runtime / "pool"
    require(command.is_file() and not command.is_symlink(), "Poolside managed launcher missing")

    link = Path(env["HOME"]) / ".docker/run/docker.sock"
    link.parent.mkdir(parents=True, mode=0o700)
    link.symlink_to(socket)
    # The gateway still resolves and verifies the entire installed runtime.
    return {"command": str(command), "args": ["acp"]}, {
        "version": VERSION, "platform": artifact["docker_platform"], "revision": source.name,
        "entry": command.name, "native_sha256": native_digest,
        "model_protocol": release["model_protocol"],
    }


def linux_container_binds(workspace, projected_skills):
    containers = docker("ps", "-q")
    require(containers.returncode == 0, "could not list live Poolside containers")
    required = [workspace] if projected_skills is None else [workspace, projected_skills]
    sandbox_image = json.loads(LINUX_CONTRACT.read_text())["sandbox_image"]
    for container in containers.stdout.splitlines():
        details = docker("inspect", "--type", "container",
                         "--format", "{{json .Config.Image}}\n{{json .Mounts}}", container)
        if details.returncode != 0:
            continue
        image, mounts = map(json.loads, details.stdout.splitlines())
        if not any(mount.get("Source") == str(workspace) for mount in mounts):
            continue
        require(image in {sandbox_image, f"docker.io/library/{sandbox_image}"},
                "Poolside sandbox did not select the fixed image")
        for path in required:
            bindings = [mount for mount in mounts if mount.get("Source") == str(path)]
            require(len(bindings) == 1 and bindings[0].get("Type") == "bind"
                    and bindings[0].get("Destination") == str(path) and bindings[0].get("RW") is False,
                    "Poolside container lost a read-only same-path bind")
        require(not any(Path(mount.get("Source", "/")).is_socket() for mount in mounts),
                "Poolside tool container received a host socket")
        return
    raise AssertionError("Poolside Docker sandbox missing a live workspace bind")


@contextmanager
def mcp_server(workspace, attachment, audit):
    with sandboxed_stdio(workspace, attachment, audit) as server:
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


def prepare_workspace(workspace):
    outside = workspace.parent / "outside"
    outside.mkdir()
    (outside / "SKILL.md").write_text(
        "---\nname: outside-poolside-probe\ndescription: Outside sentinel\n---\nignored\n")
    (workspace / NATIVE_DIR / "outside-poolside-probe").symlink_to(outside)


class Probe(ChatProbe):
    def response(self, body):
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if not tools:
            return {"content": "Poolside fixture session"}, "stop"
        require(body["model"] == MODEL, "unexpected Poolside model")
        self.requests += 1
        require(tools.keys() <= {"read", "write", "skill", MCP_TOOL},
                "Poolside exposed a tool outside the reviewed catalog")
        skills = self.fixtures["skills"]
        if self.requests == 1:
            envelope = json.dumps(body)
            catalog = tools.get("skill", {}).get("description", "")
            require("outside-poolside-probe" not in catalog, "Poolside discovered a skill outside the workspace")
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog) == visible, f"unexpected Poolside {kind} skill catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Poolside exposed skill content before activation")
            require(self.fixtures["mcp"] not in envelope, "Poolside exposed MCP data before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected Poolside MCP catalog")
            return self.activate(tools, "native")

        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"])
                   for row in body["messages"] if row["role"] == "tool"]
        result = correlated_result(results, call_id)["content"]
        self.completed[step] = call_id
        if step in ("native", "selected"):
            skill = skills[step]
            require(skill["body"] in str(result) and skill["attachment"] not in str(result),
                    f"unexpected Poolside {step} activation body")
            text = result if isinstance(result, str) else "\n".join(block["text"] for block in result)
            path_match = re.search(r"^Directory: (.+)$", text, re.M)
            require(path_match is not None, "Poolside activation omitted the directory")
            directory = Path(path_match[1])
            require(directory.is_absolute() and directory.name == skill["name"],
                    "Poolside activated an unexpected skill path")
            if step == "native":
                require(directory.resolve() == skill["path"].resolve(),
                        "Poolside activated an unexpected native directory")
            else:
                home = directory.parent.parent.parent
                require(home.name == "home" and home.parent.name.startswith("shiju-poolside-")
                        and directory.parent == home / NATIVE_DIR and directory != skill["path"],
                        "Poolside selected skill bypassed the private session view")
                self.projected_skills = directory.parent
            return self.call(tools, step + "_read", "read",
                             {"path": str(directory / "references/value.txt")})
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in str(result) and re.search(r"reject|denied|cancel", str(result), re.I))
                    if self.rejected else value in str(result), "unexpected Poolside MCP continuation")
        else:
            require(skills[step.removesuffix("_read")]["attachment"] in str(result),
                    f"Poolside {step} did not return the fixture attachment")
            if platform.system() == "Linux":
                workspace = skills["native"]["path"].parents[2].resolve()
                linux_container_binds(workspace, self.projected_skills if step == "selected_read" else None)
        if self.mounted and step == "native_read":
            return self.activate(tools, "selected")
        if self.mounted and step == "selected_read":
            delta, finish = self.call(tools, "mcp", MCP_TOOL, {})
            # Poolside ends the turn immediately on refusal, with no model continuation.
            return dict(delta, content="Checking the read-only service."), finish
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def observe_terminal(self, events):
        if not self.rejected or self.pending is None or self.pending[0] != "mcp":
            return False
        _, name, _, call_id = self.pending
        completion = [row for row in events if row.get("type") == "chat.tool"
                      and row.get("kind") == "complete" and row.get("call_id") == call_id]
        if len(completion) != 1 or completion[0].get("status") != "failed":
            return False
        payload = completion[0].get("payload", {})
        error = str(payload.get("error"))
        if payload.get("title") != name or (
            error != "Permission denied" and "user denied MCP tool" not in error
        ):
            return False
        self.completed["mcp"] = call_id
        self.pending = None
        return True

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == "mcp:coexistence:read_probe"
                and metadata.get("arguments") == {})
