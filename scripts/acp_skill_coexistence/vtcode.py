"""Locate VT Code's managed ACP skill discovery boundary."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

from . import correlated_result
from .opencode import Probe as ChatProbe

VERSION = "0.183.0"
REVISION = f"vtcode-{VERSION}-model-source-v2"
MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"  # The managed gateway projects host stdio onto private HTTP.
NATIVE_DIR = ".agents/skills"
READ_ONLY_BASELINE = "list_files"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed VT Code platform")
    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(manifest.read_text())["vtcode"]
    require(contract["version"] == VERSION, "unexpected VT Code contract version")
    name = contract["bin"]
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = Path(os.environ.get("ACP_VTCODE_REVIEWED_BIN", base / "acp-runtime" / REVISION / name))
    require(source.is_file(), "reviewed VT Code patched executable unavailable")
    entry = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION / name
    entry.parent.mkdir(parents=True, mode=0o700)
    shutil.copy2(source, entry)
    sha256 = hashlib.sha256()
    with entry.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            sha256.update(block)
    digest = sha256.hexdigest()
    require(digest == contract["files"][name], "VT Code patched executable digest mismatch")
    try:
        version = subprocess.run([str(entry), "--version"], env=env,
                                 capture_output=True, timeout=15)
    except subprocess.TimeoutExpired:
        raise AssertionError("VT Code executable version check timed out") from None
    require(version.returncode == 0 and version.stdout.splitlines()[:1] == [b"vtcode-core 0.183.0"], "unexpected VT Code executable version")
    return {"command": str(entry), "args": ["acp"]}, {
        "version": VERSION, "revision": REVISION, "platform": "darwin-arm64", "sha256": digest,
    }


class Probe(ChatProbe):
    def response(self, body):
        require(body["model"] == MODEL, "unexpected VT Code model")
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if not tools:
            return {"content": "Fixture session"}, "stop"
        self.requests += 1
        native = self.fixtures["skills"]["native"]
        if self.requests == 1:
            envelope = json.dumps(body)
            for skill in self.fixtures["skills"].values():
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "VT Code exposed skill content before activation")
                require(skill["name"] not in envelope, "VT Code now advertises skill metadata")
            require(self.fixtures["mcp"] not in envelope, "VT Code exposed MCP data before invocation")
            require(tools.keys() <= {"read_file", "list_files", "code_search", "apply_patch"},
                    "VT Code baseline tool catalog changed")
            # A known fixture path establishes ordinary native tool execution only.
            return self.call(tools, "ordinary_list", READ_ONLY_BASELINE, {"path": NATIVE_DIR})

        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"])
                   for row in body["messages"] if row["role"] == "tool"]
        correlated_result(results, call_id, native["name"])
        self.completed[step] = call_id
        self.pending = None
        self.failure = "VT Code ACP model request lacks a skill catalog and native activation tool"
        return {"content": "VTCODE_LIST_BASELINE_DONE"}, "stop"

    def observe_terminal(self, events):
        if self.mounted and any(
            row.get("type") == "chat.error"
            and "vtcode: no verified skill directory mount branch" in str(row.get("error", ""))
            for row in events
        ):
            self.failure = "VT Code host rejected the unverified skill mount before model inference"
        return False

    def approves(self, _metadata):
        return False


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated VT Code model request")
            require(self.path == "/v1/chat/completions", "unexpected VT Code model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid VT Code model request size")
            body = json.loads(self.rfile.read(size))
            require(body.get("stream") is False, "unexpected VT Code model streaming mode")
            message, finish = probe.response(body)
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid VT Code Chat Completions envelope")
            self.send_error(422, "probe rejected model request")
            return
        data = json.dumps({"id": "chatcmpl-fixture", "object": "chat.completion",
            "model": body["model"], "choices": [{"index": 0, "finish_reason": finish,
            "message": dict(message, role="assistant")}]}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
