"""Verify TRAE's native skill and stdio MCP tools through Chat Completions."""

import hashlib
import json
from pathlib import Path
import platform
import re
import secrets
import shutil
import struct
import subprocess
import sys

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

from . import correlated_result

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = "../home/.agents/skills"
VERSION = "0.121.3"
REVISION = f"trae-{VERSION}-model-source-v1"
MCP_TOOL = "mcp__coexistence__read_probe"
MANAGED = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"


def launch(env):
    system, machine = platform.system(), platform.machine()
    linux = system == "Linux" and machine in ("x86_64", "aarch64")
    require(linux or (system, machine) == ("Darwin", "arm64"), "unreviewed TRAE platform")
    manifest = MANAGED / "contracts.json"
    contract = json.loads(manifest.read_text())["trae"]
    require(contract["version"] == VERSION, "unexpected TRAE contract version")
    revision = f"trae-{VERSION}-linux-model-source-v1" if linux else REVISION
    target = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision
    target.parent.mkdir()
    assets = Path(env["TMPDIR"]) / "trae-assets"
    assets.mkdir(mode=0o700)
    shutil.copy2(manifest, assets)
    shutil.copytree(ROOT / "patches/acp-model-source/trae", assets / "trae")
    result = subprocess.run(
        [sys.executable, str(MANAGED / "install.py"), str(assets), str(target), "trae"],
        env=env, capture_output=True, timeout=180)
    require(result.returncode == 0,
            f"TRAE reviewed session-model installation failed for {system}-{machine}")
    files = ("native/trae-cli", "native/trae-local", "traecli") if linux else contract["files"]
    hashes = {file: hashlib.sha256((target / file).read_bytes()).hexdigest() for file in files}
    evidence = {"platform": system + "-" + machine, "version": VERSION, "revision": revision,
                "archive_sha256": contract["archive_sha256"], "sha256": hashes}
    args = ["acp", "serve"]
    if linux:
        contract = json.loads((assets / "trae/linux.json").read_text())
        require(contract["version"] == VERSION, "unexpected TRAE Linux contract version")
        require(contract["args"] == args, "unexpected TRAE Linux ACP arguments")
        expected = contract["targets"][machine]
        archive_sha256 = hashlib.sha256((assets / "trae.tar.gz").read_bytes()).hexdigest()
        require(archive_sha256 == expected["archive_sha256"], "TRAE Linux archive checksum mismatch")
        require(hashes["native/trae-cli"] == expected["executable_sha256"],
                "TRAE Linux source executable checksum mismatch")
        require(hashes == expected.get("session_model_files"),
                "TRAE Linux session-model files have no matching reviewed digests")
        with (target / "native/trae-cli").open("rb") as source:
            header = source.read(64)
        require(len(header) == 64 and header[:7] == b"\x7fELF\x02\x01\x01",
                "TRAE Linux source must be a little-endian ELF64 executable")
        machine_id = struct.unpack_from("<H", header, 18)[0]
        require(machine_id == expected["elf_machine"], "TRAE Linux ELF machine mismatch")
        evidence.update(archive_sha256=archive_sha256, elf_machine=machine_id,
                        entry=contract["entry"], args=args)
    else:
        require(hashes == contract["files"], "TRAE artifact digest mismatch")
    command = target / (contract["entry"] if linux else contract["bin"])
    result = subprocess.run([str(command), "--version"], env=env,
                            capture_output=True, timeout=15)
    require(result.returncode == 0
            and result.stdout.splitlines()[:1] == [f"trae-cli version {VERSION}".encode()],
            "unexpected TRAE runtime version")
    return {"command": str(command), "args": args}, evidence


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.failure = None
        self.pending = None

    def response(self, body):
        require(body["model"] == MODEL and body.get("stream") is True,
                "unexpected TRAE Chat Completions contract")
        tools = {row["function"]["name"]: row["function"] for row in body.get("tools", [])}
        self.requests += 1
        require("Read" in tools, "TRAE native Read tool is absent")
        require((MCP_TOOL in tools) == self.mounted, "unexpected TRAE stdio MCP catalog")
        skills = self.fixtures["skills"]
        if self.requests == 1:
            envelope = json.dumps(body)
            require("Skill" in tools, "TRAE did not discover a native skill")
            names = tools["Skill"]["parameters"]["properties"]["skill"]["enum"]
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in names) == visible, f"unexpected TRAE {kind} skill catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "TRAE skill content appeared before activation")
            require(self.fixtures["mcp"] not in envelope, "TRAE MCP value appeared before invocation")
            return self.call(tools, "native", "Skill", {"skill": skills["native"]["name"]})

        step, _, call_id = self.pending
        results = [dict(message, tool_use_id=message["tool_call_id"])
                   for message in body["messages"] if message["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        skill = skills.get(step.removesuffix("_read"))
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and re.search(r"reject|denied|cancel", result, re.I))
                    if self.rejected else value in result, "unexpected TRAE MCP continuation")
        else:
            value = skill["attachment"] if step.endswith("_read") else skill["body"]
            require(value in result, f"missing TRAE {step} content")
        self.completed[step] = call_id

        if step in ("native", "selected"):
            match = re.search(r"Base directory for this skill: ([^\r\n]+)", result)
            require(match is not None, "TRAE skill activation omitted its directory")
            base = Path(match[1].strip())
            if step == "native":
                require(base == skill["path"].resolve(), "unexpected TRAE native directory")
            else:
                require(base.is_absolute() and base.name == skill["name"]
                        and base.parent.name == "skills"
                        and base.parent.parent.name.startswith("shiju-acp-launch-"),
                        "TRAE selected skill did not use the private skills directory")
            return self.call(tools, step + "_read", "Read",
                             {"file_path": str(base / "references/value.txt")})
        if self.mounted and step == "native_read":
            return self.call(tools, "selected", "Skill", {"skill": skills["selected"]["name"]})
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def call(self, tools, step, name, arguments):
        schema = tools[name]["parameters"]
        require(set(schema.get("required", ())) <= arguments.keys() <= schema.get("properties", {}).keys(),
                f"unexpected TRAE {step} tool schema")
        call_id = "call_" + secrets.token_hex(12)
        self.pending = (step, name, call_id)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def observe_terminal(self, events):
        if not self.rejected or not self.pending or self.pending[:2] != ("mcp", MCP_TOOL):
            return False
        call_id = self.pending[2]
        calls = [event for event in events if event.get("type") == "chat.tool"
                 and event.get("call_id") == call_id
                 and event.get("payload", {}).get("title") == "read_probe"]
        starts = [event for event in calls if event.get("kind") == "start"]
        ends = [event for event in calls if event.get("kind") == "complete"]
        if len(starts) != 1 or len(ends) != 1:
            return False
        if ends[0]["payload"].get("output") != "Tool call rejected by user":
            return False
        self.completed["mcp"] = call_id
        return True

    def approves(self, metadata):
        return (self.pending is not None and self.pending[:2] == ("mcp", MCP_TOOL)
                and metadata.get("title") == MCP_TOOL and metadata.get("arguments") == {})


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.path == "/v1/chat/completions", "unexpected TRAE model protocol")
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated TRAE model request")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid TRAE model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, IndexError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid TRAE model envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        # TRAE ends the turn immediately after a native permission rejection.
        # Keep the model's ordinary pre-tool text available to that terminal turn.
        if probe.pending is not None and probe.pending[0] == "mcp" and finish == "tool_calls":
            self.chunk(body["model"], {"role": "assistant", "content": "Checking the read-only fixture."}, None)
        self.chunk(body["model"], dict(delta, role="assistant"), None)
        self.chunk(body["model"], {}, finish)
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()
