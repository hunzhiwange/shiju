"""Exercise the pinned OMP skill scanner, read tool, and approved HTTP bridge."""
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
from .cortex_code import mcp_server
from .opencode import Handler, Probe as ChatProbe

MODEL = "selected-model"
MODEL_REASONING = True
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".omp/skills"
VERSION = "18.22.0"
REVISION = f"omp-{VERSION}-model-source-v1"
MCP_TOOL = "mcp__coexistence_read_probe"


def launch(env):
    system, machine = platform.system(), platform.machine()
    if system == "Linux" and machine in ("x86_64", "aarch64"):
        manifest = ROOT / "patches/acp-model-source/omp/linux.json"
        contract = json.loads(manifest.read_text())
        artifact = contract["targets"][f"{machine}-unknown-linux-gnu"]
        platform_name = "linux-x64" if machine == "x86_64" else "linux-arm64"
        expected_digest = artifact["sha256"]
        revision = f"{REVISION}-{platform_name}"
        args = contract["args"]
    else:
        require((system, machine) == ("Darwin", "arm64"), "unreviewed OMP platform")
        manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
        contract = json.loads(manifest.read_text())["omp"]
        platform_name = "darwin-arm64"
        expected_digest = contract["files"]["omp"]
        revision = REVISION
        args = ["acp"]
    require(contract["version"] == VERSION and contract["bin"] == "omp" and args == ["acp"],
            "unexpected OMP contract version or entry")
    config = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = Path(os.environ.get("ACP_OMP_REVIEWED_BIN", config / "acp-runtime" / revision / "omp"))
    require(source.is_file(), "reviewed OMP executable unavailable")
    entry = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision / "omp"
    entry.parent.mkdir(parents=True, mode=0o700)
    shutil.copy2(source, entry)
    sha256 = hashlib.sha256()
    with entry.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            sha256.update(block)
    digest = sha256.hexdigest()
    require(digest == expected_digest, "OMP executable digest mismatch")
    try:
        version = subprocess.run([str(entry), "--version"], env=env, capture_output=True, timeout=15)
    except subprocess.TimeoutExpired:
        raise AssertionError("OMP version check timed out") from None
    require(version.returncode == 0 and version.stdout.strip().decode() == f"omp/{VERSION}",
            "unexpected OMP executable version")
    return {"command": str(entry), "args": args}, {
        "version": VERSION, "platform": platform_name, "sha256": digest, "revision": revision,
        "entry": entry.name, "model_endpoint": "/v1/chat/completions",
    }


def prepare_workspace(workspace):
    # In the managed route, project custom roots cannot widen the selection.
    (workspace / ".omp/config.yml").write_text(json.dumps({
        "skills": {"customDirectories": [str(workspace / ".shiju/skills")]},
    }))


class Probe(ChatProbe):
    def response(self, body):
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if "read" not in tools:
            return {"content": "OMP fixture session"}, "stop"
        require(body["model"] == MODEL, "unexpected OMP model")
        self.requests += 1
        skills = self.fixtures["skills"]
        messages = body["messages"]
        if self.requests == 1:
            require({"read", "write", "edit"}.issubset(tools), "OMP native tool inventory missing")
            catalog = "\n".join(str(row["content"]) for row in messages if row["role"] == "system")
            catalogs = re.findall(r"<skills>(.*?)</skills>", catalog, re.S)
            listed = re.findall(r"^- ([^:\n]+): ", "\n".join(catalogs), re.M)
            envelope = json.dumps(body)
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "OMP included skill content before activation")
                require(listed.count(skill["name"]) == int(visible), f"unexpected OMP {kind} catalog")
            require(self.fixtures["mcp"] not in envelope, "OMP included MCP data before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected OMP MCP inventory")
            return self.read(tools, "native", "native")

        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"])
                   for row in messages if row["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and "denied" in result.lower())
                    if self.rejected else value in result, "unexpected OMP MCP continuation")
        else:
            skill = skills[step.removesuffix("_read")]
            expected = skill["attachment"] if step.endswith("_read") else skill["body"]
            require(expected in result, f"OMP did not return {step} content")
            if step in ("native", "selected"):
                require(skill["attachment"] not in result, "OMP inlined skill attachment data")
        self.completed[step] = call_id
        if step in ("native", "selected"):
            return self.read(tools, step + "_read", step, "/references/value.txt")
        if self.mounted and step == "native_read":
            return self.read(tools, "selected", "selected")
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {"i": "Read the MCP fixture"})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def read(self, tools, step, kind, suffix=""):
        name = self.fixtures["skills"][kind]["name"]
        return self.call(tools, step, "read", {"i": "Read the fixture", "path": f"skill://{name}{suffix}"})

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == "mcp__coexistence__read_probe"
                and metadata.get("arguments") in (None, {}))
