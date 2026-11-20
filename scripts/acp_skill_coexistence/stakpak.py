"""Probe the reviewed Stakpak ACP skill discovery boundary."""
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

VERSION = "0.3.88"
REVISION = f"stakpak-{VERSION}-model-source-v3"
MODEL = "deepseek-v4.1-flash-1017"
MODEL_OUTPUT_LIMIT = 16_000
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
MCP_TOOL = "coexistence__read_probe"
NATIVE_DIR = ".stakpak/skills"
VIEW_TOOL = "stakpak__view"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Stakpak platform")
    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(manifest.read_text())["stakpak"]
    require(contract["version"] == VERSION, "unexpected Stakpak contract version")
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = Path(os.environ.get("ACP_STAKPAK_REVIEWED_DIR", base / "acp-runtime" / REVISION))
    require(source.is_dir(), "reviewed Stakpak runtime unavailable")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    runtime.mkdir(parents=True, mode=0o700)
    for name, expected in contract["files"].items():
        entry = runtime / name
        shutil.copy2(source / name, entry)
        digest = hashlib.sha256()
        with entry.open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(block)
        require(digest.hexdigest() == expected, f"Stakpak artifact digest mismatch: {name}")
    command = runtime / contract["bin"]
    try:
        version = subprocess.run([str(command), "--version"], env=env,
                                 capture_output=True, timeout=15)
    except subprocess.TimeoutExpired:
        raise AssertionError("Stakpak version check timed out") from None
    require(version.returncode == 0 and version.stdout.strip() == f"stakpak {VERSION}".encode(),
            "unexpected Stakpak executable version")
    return {"command": str(command), "args": ["acp"]}, {
        "version": VERSION, "revision": REVISION, "platform": "darwin-arm64",
        "sha256": contract["files"][contract["bin"]], "files": contract["files"],
    }


class Probe(ChatProbe):
    def response(self, body):
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if not tools:
            return {"content": "Stakpak fixture session"}, "stop"
        require(body["model"] == MODEL, "unexpected Stakpak model")
        self.requests += 1
        native = self.fixtures["skills"]["native"]
        if self.requests == 1:
            envelope = json.dumps(body)
            for skill in self.fixtures["skills"].values():
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Stakpak exposed skill content before activation")
                require(skill["name"] not in envelope, "Stakpak now advertises skill metadata")
            require(self.fixtures["mcp"] not in envelope, "Stakpak exposed MCP data before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected Stakpak MCP catalog")
            require(not any(name.rsplit("__", 1)[-1] == "load_skill" for name in tools),
                    "Stakpak now advertises load_skill and needs a new review")
            # Use the real read path only to establish a baseline. The known
            # fixture path does not demonstrate native discovery or activation.
            return self.call(tools, "ordinary_read", VIEW_TOOL,
                             {"path": str(native["path"] / "references/value.txt")})

        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"])
                   for row in body["messages"] if row["role"] == "tool"]
        correlated_result(results, call_id, native["attachment"])
        self.completed[step] = call_id
        self.pending = None
        self.failure = "Stakpak ACP model request lacks a native skill catalog and activation tool"
        return {"content": "STAKPAK_READ_BASELINE_DONE"}, "stop"

    def observe_terminal(self, events):
        if self.mounted and any(
            row.get("type") == "chat.error"
            and "stakpak: no verified skill directory mount branch" in str(row.get("error", ""))
            for row in events
        ):
            self.failure = "Stakpak host rejected the unverified skill mount before model inference"
        return False

    def approves(self, _metadata):
        return False
