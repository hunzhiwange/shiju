"""Probe the pinned Pi ACP and native RPC with native reads and read-only stdio MCP."""

from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess
import threading
import time
from xml.etree import ElementTree

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

from . import correlated_result

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".pi/skills"
MCP_TOOL = "coexistence__read_probe"
VERSION = "0.0.22"
ENGINE_SHA256 = "e959f463b06ddd15ed882783ac02f39ecaeef950ef967da86380f39dda6595fa"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Pi platform")
    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(manifest.read_text())["pi"]
    require(contract["version"] == VERSION and contract["engine_version"] == "0.73.1",
            "unexpected Pi contract versions")
    config = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    relative = Path("node_modules") / contract["package"]
    for source in sorted((config / "acp-runtime").glob(f"pi-{VERSION}-model-source-v*"), reverse=True):
        package = source / relative
        if not (package / contract["bin"]).is_file():
            continue
        if all((package / name).is_file()
               and hashlib.sha256((package / name).read_bytes()).hexdigest() == digest
               for name, digest in contract["files"].items()):
            break
    else:
        raise AssertionError("reviewed Pi runtime unavailable")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / source.name
    shutil.copytree(source, runtime, symlinks=True, ignore=shutil.ignore_patterns("pi-source"))
    package = runtime / relative
    engine = runtime / "node_modules" / contract["engine"]
    hashes = {name: hashlib.sha256((package / name).read_bytes()).hexdigest()
              for name in contract["files"]}
    require(hashes == contract["files"], "Pi adapter digest mismatch")
    for directory, name, version in ((package, contract["package"], VERSION),
                                     (engine, contract["engine"], contract["engine_version"])):
        metadata = json.loads((directory / "package.json").read_text())
        require((metadata["name"], metadata["version"]) == (name, version),
                "unexpected Pi package identity")
    native = engine / "dist/cli.js"
    require(hashlib.sha256(native.read_bytes()).hexdigest() == ENGINE_SHA256,
            "Pi engine digest mismatch")
    version_env = dict(env, PI_OFFLINE="1", PI_SKIP_VERSION_CHECK="1")
    version = subprocess.run([str(native), "--version"], env=version_env,
                             capture_output=True, timeout=15)
    require(version.returncode == 0 and (version.stdout + version.stderr).strip() == b"0.73.1",
            "unexpected Pi executable version")
    return {"command": str(package / contract["bin"])}, {
        "version": VERSION, "engine_version": contract["engine_version"], "revision": source.name,
        "sha256": hashes, "engine_sha256": ENGINE_SHA256,
    }


@contextmanager
def mcp_server(workspace, attachment, audit):
    """Observe calls in the runner's ephemeral writable scratch without opening the sandbox."""
    for name in ("acp_mount_integration.py", "verify_acp_integration.py"):
        shutil.copy2(ROOT / "scripts" / name, workspace / name)
    value = workspace / ".pi-mcp-value"
    shutil.copy2(attachment, value)
    marker = "pi-probe-" + secrets.token_hex(16)
    ack = workspace / marker
    runner = workspace / ".pi-mcp-probe.py"
    runner.write_text("""import json
from pathlib import Path
import sys
import time

from acp_mount_integration import rpc

value, ack = map(Path, sys.argv[1:])
marker = Path.home() / ack.name
for line in sys.stdin:
    request = json.loads(line)
    if request.get("method") == "tools/call":
        sequence = int(ack.read_text()) + 1
        response = rpc(request, value, marker, "stdio")
        end = time.monotonic() + 5
        while int(ack.read_text()) < sequence:
            if time.monotonic() > end:
                raise RuntimeError("missing host audit acknowledgment")
            time.sleep(0.02)
    else:
        response = rpc(request, value, marker, "stdio")
    if response:
        print(json.dumps(response), flush=True)
""")
    ack.write_text("0")
    stop = threading.Event()

    def collect():
        seen = set()
        count = 0
        while not stop.wait(0.02):
            for path in Path("/tmp").glob("shiju-cortex-mcp-*/" + marker):
                if path in seen or not path.is_file():
                    continue
                if path.read_text() != "stdio\n":
                    continue
                seen.add(path)
                with audit.open("a") as report:
                    report.write("stdio\n")
                count += 1
                update = ack.with_suffix(".next")
                update.write_text(str(count))
                update.replace(ack)

    observer = threading.Thread(target=collect)
    observer.start()
    try:
        python = Path(subprocess.check_output(["xcrun", "--find", "python3"], text=True).strip()).resolve()
        framework = python.parents[1] / "Resources/Python.app/Contents/MacOS/Python"
        # The framework's bin/python3 shim needs posix_spawn, denied by the reviewed sandbox.
        yield {"name": "coexistence", "command": str(framework if framework.is_file() else python),
               "args": ["-B", str(runner), str(value), str(ack)], "env": []}
    finally:
        stop.set()
        observer.join()


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.pending = None
        self.locations = {}
        self.failure = None

    def catalog(self, body, tools):
        require(set(tools) == ({"read", MCP_TOOL} if self.mounted else {"read"}),
                "restricted Pi exposed an unexpected tool")
        system = "\n".join(str(message["content"]) for message in body["messages"]
                           if message["role"] == "system")
        catalog = {}
        for section in re.findall(r"<available_skills>.*?</available_skills>", system, re.S):
            for entry in ElementTree.fromstring(section).findall("skill"):
                name = entry.findtext("name")
                require(name not in catalog, "duplicate Pi skill catalog entry")
                catalog[name] = Path(entry.findtext("location"))
        envelope = json.dumps(body)
        for kind, skill in self.fixtures["skills"].items():
            location = catalog.get(skill["name"])
            visible = kind == "native" or (kind == "selected" and self.mounted)
            require((location is not None) == visible, f"unexpected Pi {kind} skill catalog")
            require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                    "Pi skill content appeared before reading")
            if location is None:
                continue
            expected = skill["path"] / "SKILL.md"
            require(location.is_absolute() and location.resolve() == expected.resolve(),
                    f"unexpected Pi {kind} skill location")
            if kind == "selected":
                require(location.parent.parent.parent.name.startswith("shiju-session-skills-"),
                        "Pi selected skill bypassed the session view")
            self.locations[kind] = location
        require(self.fixtures["mcp"] not in envelope, "MCP value appeared before invocation")
        require((MCP_TOOL in tools) == self.mounted, "unexpected Pi MCP tool catalog")

    def response(self, body):
        require(body["model"] == MODEL, "unexpected Pi model")
        tools = {tool["function"]["name"]: tool["function"] for tool in body.get("tools", [])}
        if not tools:
            return {"content": "Fixture session"}, "stop"
        self.requests += 1
        if self.requests == 1:
            self.catalog(body, tools)
            return self.read(tools, "native", self.locations["native"])

        step, call_id = self.pending
        results = [dict(message, tool_use_id=message["tool_call_id"]) for message in body["messages"]
                   if message["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and re.search(r"reject|denied|cancel", result, re.I))
                    if self.rejected else value in result, "unexpected Pi MCP continuation")
        else:
            kind = step.removesuffix("_read")
            field = "attachment" if step.endswith("_read") else "body"
            require(self.fixtures["skills"][kind][field] in result,
                    f"unexpected Pi {step} read result")
        self.completed[step] = call_id
        if step in self.locations:
            return self.read(tools, step + "_read", self.locations[step].parent / "references/value.txt")
        if self.mounted and step == "native_read":
            return self.read(tools, "selected", self.locations["selected"])
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def read(self, tools, step, path):
        properties = tools.get("read", {}).get("parameters", {}).get("properties", {})
        require("path" in properties, "Pi native read tool missing its path")
        return self.call(tools, step, "read", {"path": str(path)})

    def call(self, tools, step, name, arguments):
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= arguments.keys() <= schema["properties"].keys(),
                f"Pi {step} tool schema changed")
        call_id = "probe" + secrets.token_hex(12)
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
                    "unauthenticated Pi model request")
            require(self.path == "/v1/chat/completions", "unexpected Pi model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Pi model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, ElementTree.ParseError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Pi Chat Completions envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        message = dict(delta, role="assistant")
        if body.get("stream"):
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            self.chunk(body["model"], message, None)
            self.chunk(body["model"], {}, finish)
            self.wfile.write(b"data: [DONE]\n\n")
        else:
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"id": "chatcmpl-fixture", "object": "chat.completion",
                "model": body["model"], "choices": [{"index": 0, "finish_reason": finish,
                                                    "message": message}]}).encode())
