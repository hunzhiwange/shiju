"""Verify the reviewed Cortex Code Chat transport and native file reads."""
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
import subprocess
import sys
import threading
from xml.etree import ElementTree

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

from . import correlated_result

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
CHAT_ENDPOINT = f"{API_PATH}/chat/completions"
TRANSPORT = "stdio"
NATIVE_DIR = ".claude/skills"
VERSION = "1.2.46+175654.6f8561fd271f"
REVISION = f"cortex-code-{VERSION}-model-source-v5"
MCP_TOOL = "shiju_mcp_coexistence__read_probe"
DISABLED_SKILL = "disabled-cortex-probe"


def launch(env):
    if platform.system() == "Linux":
        return launch_linux(env)
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Cortex Code platform")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"
                           / "contracts.json").read_text())["cortex-code"]
    require(contract["version"] == VERSION and contract["bin"] == "cortex",
            "unexpected Cortex Code installation contract")
    source = Path(os.environ.get("ACP_CORTEX_REVIEWED_DIR",
                                 Path.home() / ".local/share/shiju/acp-runtime" / REVISION))
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime"
    runtime.mkdir(mode=0o700)
    target = runtime / REVISION
    shutil.copytree(source, target, symlinks=True)
    for name, digest in contract["files"].items():
        artifact = target / name
        require(artifact.is_file() and not artifact.is_symlink()
                and hashlib.sha256(artifact.read_bytes()).hexdigest() == digest,
                f"reviewed Cortex Code {name} digest mismatch")
    version = subprocess.run([str(target / "cortex"), "--version"], env=env,
                             capture_output=True, timeout=15)
    require(version.returncode == 0 and version.stdout.strip() == b"Cortex Code v1.2.46"
            and (target / "cortex-version.txt").read_text().strip() == VERSION,
            "unexpected Cortex Code version")
    return {"command": str(target / "cortex"), "args": ["acp", "serve"]}, {
        "version": VERSION, "revision": REVISION, "sha256": contract["files"],
    }


def launch_linux(env):
    manifest = json.loads((ROOT / "patches/acp-model-source/cortex-code"
                           / "linux-artifacts.json").read_text())
    target = manifest["targets"].get(platform.machine())
    require(target is not None and manifest["version"] == VERSION,
            "unreviewed Cortex Code Linux artifact")
    directory = os.environ.get("ACP_CORTEX_REVIEWED_DIR")
    require(directory, "set ACP_CORTEX_REVIEWED_DIR to the private Linux Cortex fixture")
    source = Path(directory).resolve(strict=True)
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime"
    runtime.mkdir(mode=0o700)
    installation = runtime / source.name
    shutil.copytree(source, installation, symlinks=True)
    entry = installation / manifest["entry"]
    native = installation / "cortex-native"
    if not native.exists() and not native.is_symlink():
        native = entry
    require(entry.is_file() and not entry.is_symlink() and os.access(entry, os.X_OK),
            "missing or redirected Cortex Code Linux entry")
    files = [(native, "native_sha256")]
    if native != entry:
        files.append((installation / "cortex-local", "local_sha256"))
    digests = {}
    for artifact, key in files:
        require(artifact.is_file() and not artifact.is_symlink() and os.access(artifact, os.X_OK),
                "missing or redirected Cortex Code Linux executable")
        with artifact.open("rb") as handle:
            digest = hashlib.sha256()
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
        digests[key] = digest.hexdigest()
        require(digests[key] == target[key], f"unexpected Cortex Code Linux {artifact.name} digest")
    version_file = installation / "cortex-version.txt"
    require(version_file.is_file() and not version_file.is_symlink()
            and version_file.read_text().strip() == VERSION,
            "unexpected Cortex Code Linux version")
    # The gateway must validate the reviewed entry itself before it can start a process.
    return {"command": str(entry), "args": manifest["args"]}, {
        "version": VERSION, "entry": entry.name, **digests,
        "model_endpoint": CHAT_ENDPOINT,
    }


def prepare_workspace(workspace):
    disabled = workspace / NATIVE_DIR / DISABLED_SKILL
    disabled.mkdir()
    (disabled / "SKILL.md").write_text(
        f"---\nname: {DISABLED_SKILL}\ndescription: Disabled fixture\n"
        "disable-model-invocation: true\n---\nKeep this skill disabled.\n")


@contextmanager
def mcp_server(workspace, attachment, audit):
    # The service can read the workspace but cannot write its audit log there.
    # Serve the random value through a read-only FIFO and log successful reads
    # from the parent, outside the vendor sandbox.
    scripts = workspace / ".mcp-probe"
    scripts.mkdir(mode=0o700)
    for name in ("acp_mount_integration.py", "verify_acp_integration.py"):
        shutil.copyfile(ROOT / "scripts" / name, scripts / name)
    pipe = scripts / "value"
    os.mkfifo(pipe, 0o600)
    value = attachment.read_bytes()
    stop = threading.Event()

    def serve():
        while not stop.wait(0.05):
            try:
                fd = os.open(pipe, os.O_WRONLY | os.O_NONBLOCK)
            except OSError as error:
                if error.errno == errno.ENXIO:
                    continue
                raise
            try:
                os.write(fd, value)
                with audit.open("a") as log:
                    log.write("stdio\n")
            finally:
                os.close(fd)

    thread = threading.Thread(target=serve, daemon=True)
    thread.start()
    try:
        python = Path(sys.executable).resolve()
        # Apple's Python shim spawns a framework executable. Start that
        # executable directly because the MCP sandbox forbids child processes.
        framework = Path(sys.base_prefix) / "Resources/Python.app/Contents/MacOS/Python"
        if framework.is_file():
            python = framework.resolve()
        yield {"name": "coexistence", "command": str(python),
               "args": ["-B", str(scripts / "acp_mount_integration.py"), "--mcp-stdio",
                        str(pipe), os.devnull], "env": []}
    finally:
        stop.set()
        thread.join()


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.failure = None
        self.completed = {}
        self.pending = None
        self.paths = {}

    def response(self, body):
        require(body["model"] == MODEL and body.get("stream") is True,
                "unexpected Cortex Code Chat Completions contract")
        self.requests += 1
        tools = {row["function"]["name"]: row["function"] for row in body["tools"]}
        expected_tools = {"read", "write", "edit"}
        if self.mounted:
            expected_tools.add(MCP_TOOL)
        require(tools.keys() == expected_tools, "unexpected Cortex native or MCP tool catalog")
        skills = self.fixtures["skills"]
        if self.requests == 1:
            envelope = json.dumps(body)
            for skill in skills.values():
                require(all(skill[key] not in envelope for key in ("body", "attachment")),
                        "Cortex skill content appeared before native activation")
            require(self.fixtures["mcp"] not in envelope, "Cortex MCP value appeared before invocation")
            systems = "\n".join(str(row.get("content", "")) for row in body["messages"]
                                if row["role"] == "system")
            match = re.search(r"<available_skills>.*?</available_skills>", systems, re.S)
            require(match is not None, "Cortex Chat transport omitted the native skill catalog")
            catalog = ElementTree.fromstring(match[0])
            require(DISABLED_SKILL not in {row.findtext("name") for row in catalog.findall("skill")},
                    "Cortex advertised a skill with model invocation disabled")
            for kind, skill in skills.items():
                paths = [Path(row.findtext("location")) for row in catalog.findall("skill")
                         if row.findtext("name") == skill["name"]]
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require(len(paths) == int(visible), f"unexpected Cortex {kind} skill catalog")
                if visible:
                    path = paths[0]
                    require(path.is_absolute() and path.name == "SKILL.md"
                            and path.resolve() == skill["path"].resolve() / "SKILL.md",
                            f"unexpected Cortex {kind} catalog location")
                    require(kind == "native" or path.parent != skill["path"],
                            "Cortex bypassed the private selection")
                    self.paths[kind] = path
            return self.call(tools, "native", "read", {"file_path": str(self.paths["native"])})

        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"]) for row in body["messages"]
                   if row["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        if step == "mcp":
            expected = "no approved active tool" if self.rejected else self.fixtures["mcp"]
            require(expected in result and (not self.rejected or self.fixtures["mcp"] not in result),
                    "unexpected Cortex MCP continuation")
        else:
            skill = skills[step.removesuffix("_read")]
            require(skill["attachment" if step.endswith("_read") else "body"] in result,
                    f"missing Cortex {step} content")
        self.completed[step] = call_id
        if step in ("native", "selected"):
            return self.call(tools, step + "_read", "read",
                             {"file_path": str(self.paths[step].parent / "references/value.txt")})
        if step == "native_read" and self.mounted:
            return self.call(tools, "selected", "read", {"file_path": str(self.paths["selected"])})
        if step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def call(self, tools, step, name, arguments):
        require(name in tools, f"Cortex omitted tool for {step}")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= arguments.keys() <= schema.get("properties", {}).keys(),
                f"unexpected Cortex {step} tool schema")
        call_id = "call_" + secrets.token_hex(12)
        self.pending = (step, name, arguments, call_id)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == MCP_TOOL and metadata.get("arguments") == {})

class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.path == CHAT_ENDPOINT, "unexpected Cortex Code model protocol")
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Cortex Code model request")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Cortex Code model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, ElementTree.ParseError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Cortex Code model envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.chunk(body["model"], dict(delta, role="assistant"), None)
        self.chunk(body["model"], {}, finish)
        self.wfile.write(b"data: [DONE]\n\n")
