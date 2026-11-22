"""Exercise Kiro's reviewed V3 Responses engine and native stdio MCP."""
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
import sys
import tempfile

from acp_skill_mount_codex import responses_wire
from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

from . import correlated_result

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai-responses"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".kiro/skills"
MCP_TOOL_ID = "coexistence::read_probe"
REVISION = "kiro-3.0.0-model-source-v2"
PATCH_DIR = ROOT / "patches/acp-model-source/kiro"
MANAGED_DIR = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"


def launch(env):
    if platform.system() == "Linux":
        return launch_linux(env)
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Kiro platform")
    manifest = MANAGED_DIR / "contracts.json"
    contract = json.loads(manifest.read_text())["kiro"]
    require(contract["version"] == "3.0.0", "unexpected Kiro contract version")
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = base / "acp-runtime" / REVISION
    target = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    target.parent.mkdir()
    if source.is_dir():
        shutil.copytree(source, target)
    else:
        with tempfile.TemporaryDirectory() as directory:
            assets = Path(directory)
            shutil.copy2(manifest, assets)
            shutil.copytree(PATCH_DIR, assets / "kiro")
            try:
                result = subprocess.run(
                    [sys.executable, str(MANAGED_DIR / "install.py"), str(assets), str(target), "kiro"],
                    env=env, capture_output=True, timeout=300)
            except subprocess.TimeoutExpired as error:
                raise AssertionError("Kiro reviewed runtime installation timed out") from error
            require(result.returncode == 0, "Kiro reviewed runtime installation failed")
    files = {name: hashlib.sha256((target / name).read_bytes()).hexdigest()
             for name in contract["files"]}
    require(files == contract["files"], "Kiro runtime digest mismatch")
    entry = target / contract["bin"]
    try:
        version = subprocess.run([str(entry), "--version"], env=env, capture_output=True, timeout=15)
    except subprocess.TimeoutExpired as error:
        raise AssertionError("Kiro version check timed out") from error
    require(version.returncode == 0 and version.stdout.strip() == b"kiro-cli 3.0.0",
            "unexpected Kiro version")
    return {"command": str(entry), "args": ["acp"]}, {
        "version": contract["version"], "revision": REVISION, "files": files}


def launch_linux(env):
    contract = json.loads((PATCH_DIR / "linux.json").read_text())
    arch = platform.machine()
    require(arch in contract["targets"], "unreviewed Kiro Linux architecture")
    artifact = contract["targets"][arch]
    source = Path(os.environ.get("SHIJU_ACP_KIRO_RUNTIME_DIR", ""))
    require(source.is_absolute() and source.is_dir(), "set SHIJU_ACP_KIRO_RUNTIME_DIR")
    expected = {
        contract["bin"]: contract["launcher_sha256"],
        contract["native_entry"]: artifact["native_sha256"],
        contract["node_entry"]: artifact["node_sha256"],
        contract["server_entry"]: contract["server_sha256"],
    }
    target = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / contract["revision"]
    shutil.copytree(source, target)
    files = {entry: hashlib.sha256((target / entry).read_bytes()).hexdigest() for entry in expected}
    require(files == expected, "Kiro Linux runtime digest mismatch")
    entry = target / contract["bin"]
    try:
        version = subprocess.run([str(entry), "--version"], env=env, capture_output=True, timeout=15)
    except subprocess.TimeoutExpired as error:
        raise AssertionError("Kiro Linux version check timed out") from error
    require(version.returncode == 0 and version.stdout.strip() == b"kiro-cli 3.0.0",
            "unexpected Kiro Linux version")
    return {"command": str(entry), "args": contract["args"]}, {
        "version": contract["version"], "revision": contract["revision"], "arch": arch,
        "archive": artifact["artifact_url"], "files": files,
        "protocol": contract["model_protocol"], "model_path": "/v1/responses"}


@contextmanager
def mcp_server(_workspace, attachment, audit):
    script = ROOT / "scripts/acp_mount_integration.py"
    args = [str(script), "--mcp-stdio", str(attachment), str(audit)]
    if platform.system() == "Linux":
        args = ["-c", (
            "import os, sys\n"
            "if any(key in os.environ for key in "
            "('KIRO_API_KEY', 'VW_ACP_LAUNCH_REFERENCE')):\n"
            "    raise SystemExit('MCP child received managed Kiro credentials')\n"
            "os.execv(sys.executable, [sys.executable, *sys.argv[1:]])\n"
        ), *args]
    yield {"name": "coexistence", "command": sys.executable, "args": args, "env": []}


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.failure = None
        self.pending = None
        self.check_native_power = platform.system() == "Linux"

    def response(self, body):
        require(body["model"] == MODEL, "unexpected Kiro model")
        tools = {tool["name"]: tool for tool in body.get("tools", [])}
        if not tools:
            return {"content": "Fixture session"}
        self.requests += 1
        skills = self.fixtures["skills"]
        description = tools.get("disclose_context", {}).get("description", "").replace(r'\"', '"')
        names = set(re.findall(r'name: "([^"\n]+)"', description))
        for kind, skill in skills.items():
            visible = kind == "native" or (kind == "selected" and self.mounted)
            require((skill["name"] in names) == visible, f"unexpected Kiro {kind} skill catalog")
        tool_catalog = tools.get("tool_load", {}).get("description", "").splitlines()
        require(self.mounted or MCP_TOOL_ID not in tool_catalog, "unexpected Kiro stdio MCP catalog")
        if self.requests == 1:
            envelope = json.dumps(body)
            for skill in skills.values():
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Kiro skill content appeared before activation")
            require(self.fixtures["mcp"] not in envelope, "Kiro MCP content appeared before invocation")
            if self.check_native_power:
                return self.call(tools, "power", "kiro_powers", {"action": "list"})
            return self.activate(tools, "native")

        step, _, call_id = self.pending
        results = [{"tool_use_id": item["call_id"], "content": item["output"]}
                   for item in body["input"] if item.get("type") == "function_call_output"]
        result = str(correlated_result(results, call_id)["content"])
        if step == "power":
            require("No powers installed" in result, "Kiro native Powers did not return its list")
            return self.activate(tools, "native")
        if step in ("native", "selected"):
            skill = skills[step]
            require(skill["body"] in result and skill["attachment"] not in result,
                    f"unexpected Kiro {step} activation content")
            paths = re.findall(r'(/[^\s<>"\x27`\\]*' + re.escape("/" + skill["name"]) + r')', result)
            directory = next((Path(path) for path in paths if Path(path).resolve() == skill["path"]), None)
            require(directory is not None, f"Kiro {step} activation omitted its directory")
            if step == "native":
                require(directory == skill["path"], "Kiro native skill bypassed its workspace directory")
            else:
                require(directory.parent.parent.name.startswith("shiju-session-skills-"),
                        "Kiro selected skill bypassed the session view")
            self.completed[step] = call_id
            return self.call(tools, step + "_read", "read_file",
                             {"path": str(directory / "references/value.txt")})
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and re.search(r"reject|denied|cancel", result, re.I))
                    if self.rejected else value in result, "unexpected Kiro MCP continuation")
        elif step == "load":
            declaration = json.loads(result)
            loaded = declaration["loaded"]
            require(len(loaded) == 1 and not declaration["not_found"]
                    and loaded[0]["tool_id"] == MCP_TOOL_ID, "unexpected Kiro MCP declaration")
            schema = loaded[0]["input_schema"]
            require(schema["type"] == "object" and schema["properties"] == {}
                    and not schema.get("required"), "unexpected Kiro read-only MCP tool schema")
            return self.call(tools, "mcp", "tool_call", {"tool_id": loaded[0]["tool_id"], "arguments": {}})
        else:
            require(skills[step.removesuffix("_read")]["attachment"] in result,
                    f"Kiro {step} did not return its attachment")
        self.completed[step] = call_id
        if self.mounted and step == "native_read":
            return self.activate(tools, "selected")
        if self.mounted and step == "selected_read":
            require(MCP_TOOL_ID in tool_catalog, "Kiro stdio MCP discovery did not finish")
            return self.call(tools, "load", "tool_load", {"tool_ids": [MCP_TOOL_ID]})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}

    def activate(self, tools, kind):
        return self.call(tools, kind, "disclose_context", {"name": self.fixtures["skills"][kind]["name"]})

    def call(self, tools, step, name, args):
        require(name in tools, f"Kiro {step} tool missing")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", ())) <= args.keys() <= schema.get("properties", {}).keys(),
                f"Kiro {step} tool schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.pending = (step, args, call_id)
        return {"tool_calls": [{"id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(args)}}]}

    def approves(self, metadata):
        if self.pending is None:
            return False
        step, args, _ = self.pending
        # Kiro's ACP prompt reports the deferred tool's title without rawInput.
        return (step == "mcp" and args == {"tool_id": MCP_TOOL_ID, "arguments": {}}
                and metadata.get("title") == "@coexistence/read_probe"
                and metadata.get("arguments") is None)

    def approves_skill(self, metadata):
        if self.pending is None:
            return None
        step, args, _ = self.pending
        if (step in ("native", "selected") and metadata.get("title") == "Load skill: " + args["name"]
                and metadata.get("arguments") is None):
            return step


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Kiro model request")
            require(self.path == "/v1/responses", "unexpected Kiro model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Kiro model request size")
            body = json.loads(self.rfile.read(size))
            message = probe.response(body)
        except (AssertionError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Kiro Responses envelope")
            self.send_error(422, "probe rejected Kiro model request")
            return
        reply, stream = responses_wire(message, MODEL)
        data = stream if body.get("stream") else json.dumps(reply).encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream" if body.get("stream") else "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
