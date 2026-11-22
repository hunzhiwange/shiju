"""Probe the pinned OpenClaw Gateway with native reads and read-only stdio MCP."""

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
from xml.etree import ElementTree

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

from . import correlated_result
from .pi import mcp_server

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = "skills"
MCP_TOOL = "coexistence__read_probe"
VERSION = "2026.10.10"
REVISION = f"openclaw-{VERSION}-model-source-v3"


def install_linux(env, manifest, runtime, arch):
    assets = runtime.parent.parent / "openclaw-install-assets"
    assets.mkdir(mode=0o700)
    shutil.copyfile(manifest, assets / "contracts.json")
    patches = ROOT / "patches/acp-model-source/openclaw"
    shutil.copytree(patches, assets / "openclaw")
    linux = json.loads((patches / "linux.json").read_text())
    target = linux["targets"][f"{arch}-unknown-linux-gnu"]
    node = subprocess.run(["node", "--version"], env=env, capture_output=True,
                          text=True, timeout=15)
    require(node.returncode == 0 and node.stdout.strip() == "v" + linux["node_version"],
            "unexpected Linux OpenClaw Node version")
    runtime.parent.mkdir(mode=0o700)
    try:
        result = subprocess.run(
            [sys.executable, str(manifest.with_name("install.py")), str(assets),
             str(runtime), "openclaw"],
            env=dict(env, npm_config_global="false", npm_config_package_lock="true",
                     npm_config_include="optional"), capture_output=True, timeout=240)
    except subprocess.TimeoutExpired:
        raise AssertionError("private OpenClaw installation timed out") from None
    require(result.returncode == 0, "private OpenClaw installation failed")

    lock = json.loads((runtime / "package-lock.json").read_text())["packages"]
    hashes = {}
    for name, spec in target["native_packages"].items():
        relative = "node_modules/" + name
        root = runtime / relative
        metadata = json.loads((root / "package.json").read_text())
        require((metadata["name"], metadata["version"]) == (name, spec["version"])
                and lock[relative]["integrity"] == spec["integrity"],
                f"unexpected Linux OpenClaw dependency identity: {name}")
        for name_in_package, expected in spec["files"].items():
            binary = (root / name_in_package).read_bytes()
            require(binary[:6] == b"\x7fELF\x02\x01"
                    and int.from_bytes(binary[18:20], "little") == target["elf_machine"]
                    and hashlib.sha256(binary).hexdigest() == expected,
                    f"unexpected Linux OpenClaw binary: {name}")
            hashes[f"{name}/{name_in_package}"] = expected
    return hashes


def launch(env):
    target = (platform.system(), platform.machine())
    require(target == ("Darwin", "arm64") or (
        target in {("Linux", "x86_64"), ("Linux", "aarch64")}
        and platform.libc_ver()[0] == "glibc"
    ), "unsupported OpenClaw probe target")
    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(manifest.read_text())["openclaw"]
    require(contract["version"] == VERSION, "unexpected OpenClaw contract version")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    if target[0] == "Linux":
        native_hashes = install_linux(env, manifest, runtime, target[1])
    else:
        config = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
        source = config / "acp-runtime" / REVISION
        require(source.is_dir(), "reviewed OpenClaw v3 runtime unavailable")
        shutil.copytree(source, runtime, symlinks=True)
    package = runtime / "node_modules" / contract["package"]
    metadata = json.loads((package / "package.json").read_text())
    require(metadata["name"] == contract["package"] and metadata["version"] == VERSION,
            "unexpected OpenClaw package identity")
    hashes = {name: hashlib.sha256((package / name).read_bytes()).hexdigest()
              for name in contract["files"]}
    require(hashes == contract["files"], "OpenClaw artifact digest mismatch")
    entry = runtime / "node_modules/.bin/openclaw"
    require(entry.resolve() == package / contract["bin"], "unexpected OpenClaw entry target")
    version = subprocess.run([str(entry), "--version"], env=env, capture_output=True,
                             text=True, timeout=15)
    require(version.returncode == 0 and version.stdout.strip() == f"OpenClaw {VERSION} (7dc3350)",
            "unexpected OpenClaw executable version")
    artifact = {"version": VERSION, "revision": REVISION, "sha256": hashes}
    if target[0] == "Linux":
        artifact.update(entry=contract["bin"], native_sha256=native_hashes)
    return {"command": str(entry), "args": ["acp"]}, artifact


class Probe:
    def __init__(self, fixtures, mounted, rejected, *, restricted=True, skills_only=False):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.restricted = restricted
        self.expect_mcp = mounted and not skills_only
        self.requests = 0
        self.completed = {}
        self.pending = None
        self.locations = {}
        self.failure = None

    def catalog(self, body, tools):
        system = "\n".join(str(message["content"]) for message in body["messages"]
                           if message["role"] == "system")
        catalog = {}
        for section in re.findall(r"^<available_skills>\s*\n.*?^</available_skills>\s*$", system, re.M | re.S):
            for entry in ElementTree.fromstring(section).findall("skill"):
                name = entry.findtext("name")
                require(name not in catalog, "duplicate OpenClaw skill catalog entry")
                catalog[name] = Path(entry.findtext("location"))
        envelope = json.dumps(body)
        for kind, skill in self.fixtures["skills"].items():
            location = catalog.get(skill["name"])
            visible = kind == "native" or (kind == "selected" and self.mounted)
            require((location is not None) == visible, f"unexpected OpenClaw {kind} skill catalog")
            require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                    "OpenClaw skill content appeared before reading")
            if location is None:
                continue
            require(location.is_absolute() and location.resolve() == (skill["path"] / "SKILL.md").resolve(),
                    f"unexpected OpenClaw {kind} skill location")
            if kind == "selected":
                require(location.parent.parent.parent.name.startswith("shiju-session-skills-"),
                        "OpenClaw selected skill bypassed the session view")
            self.locations[kind] = location
        require(self.fixtures["mcp"] not in envelope, "MCP value appeared before invocation")
        require((MCP_TOOL in tools) == self.expect_mcp, "unexpected OpenClaw MCP tool catalog")

    def response(self, body):
        require(body["model"] == MODEL, "unexpected OpenClaw model")
        tools = {tool["function"]["name"]: tool["function"] for tool in body.get("tools", [])}
        if not tools:
            return {"content": "Fixture session"}, "stop"
        self.requests += 1
        if self.restricted:
            allowed = {"read", "skills_read"}
            if self.expect_mcp:
                allowed.add(MCP_TOOL)
            require(set(tools) <= allowed, "restricted OpenClaw tool catalog: " + ", ".join(sorted(tools)))
        if self.requests == 1:
            self.catalog(body, tools)
            boundary = self.fixtures.get("read_boundary")
            if boundary is not None:
                return self.read(tools, "outside_read_denied", boundary["path"])
            return self.read(tools, "native", self.locations["native"])

        step, call_id = self.pending
        results = [dict(message, tool_use_id=message["tool_call_id"]) for message in body["messages"]
                   if message["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        if step == "outside_read_denied":
            boundary = self.fixtures["read_boundary"]
            require(boundary["value"] not in result and re.search(r"outside|escape|denied", result, re.I),
                    "OpenClaw native reader did not enforce its workspace boundary")
            self.completed[step] = call_id
            return self.read(tools, "native", self.locations["native"])
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and re.search(r"reject|denied|cancel", result, re.I))
                    if self.rejected else value in result, "unexpected OpenClaw MCP continuation")
        else:
            kind = step.removesuffix("_read")
            field = "attachment" if step.endswith("_read") else "body"
            require(self.fixtures["skills"][kind][field] in result,
                    f"unexpected OpenClaw {step} read result")
        self.completed[step] = call_id
        if step in self.locations:
            return self.read(tools, step + "_read", self.locations[step].parent / "references/value.txt")
        if self.mounted and step == "native_read":
            return self.read(tools, "selected", self.locations["selected"])
        if self.expect_mcp and step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def read(self, tools, step, path):
        properties = tools.get("read", {}).get("parameters", {}).get("properties", {})
        key = next((key for key in ("file_path", "path") if key in properties), None)
        require(key is not None, "OpenClaw native read tool missing its path")
        return self.call(tools, step, "read", {key: str(path)})

    def call(self, tools, step, name, arguments):
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= arguments.keys() <= schema["properties"].keys(),
                f"OpenClaw {step} tool schema changed")
        call_id = "probe" + secrets.token_hex(12)
        self.pending = (step, call_id)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == "mcp__coexistence__read_probe" and metadata.get("arguments") == {})


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated OpenClaw model request")
            require(self.path == "/v1/chat/completions", "unexpected OpenClaw model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid OpenClaw model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, ElementTree.ParseError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid OpenClaw Chat Completions envelope")
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
