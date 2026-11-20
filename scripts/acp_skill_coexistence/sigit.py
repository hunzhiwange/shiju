"""Exercise the managed siGit native skill tool and sandboxed stdio MCP."""
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys

from verify_acp_integration import require
from verify_acp_model_source import ROOT

from . import correlated_result
from .cortex_code import mcp_server as stdio_fixture
from .opencode import Handler, Probe as ChatProbe

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".sigit/skills"
VERSION = "1.7.10"
REVISION = f"sigit-{VERSION}-model-source-v4"
MCP_TOOL = "mcp__coexistence__read_probe"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed siGit platform")
    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(manifest.read_text())["sigit"]
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = Path(os.environ.get("ACP_SIGIT_REVIEWED_DIR", base / "acp-runtime" / REVISION))
    require(contract["version"] == VERSION and (source / "build-receipt.json").is_file()
            and (source / "sigit").is_file(), "reviewed siGit executable unavailable")
    receipt = json.loads((source / "build-receipt.json").read_text())
    for field in ("version", "source_sha256", "patch_sha256"):
        require(receipt[field] == contract[field], "siGit build receipt mismatch")
    target = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    target.mkdir(parents=True, mode=0o700)
    entry = target / "sigit"
    shutil.copy2(source / "sigit", entry)
    shutil.copy2(source / "build-receipt.json", target / "build-receipt.json")
    digest = hashlib.sha256()
    with entry.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    require(digest.hexdigest() == receipt["sha256"], "siGit executable digest mismatch")
    version = subprocess.run([str(entry), "--version"], env=env, capture_output=True, timeout=15)
    require(version.returncode == 0 and version.stdout.strip() == b"sigit 1.7.10",
            "unexpected siGit executable version")
    return {"command": str(entry), "args": ["--acp"]}, {
        "version": VERSION, "platform": "darwin-arm64", "sha256": digest.hexdigest(),
        "revision": REVISION, "source_sha256": receipt["source_sha256"],
        "patch_sha256": receipt["patch_sha256"],
    }


@contextmanager
def mcp_server(workspace, attachment, audit):
    with stdio_fixture(workspace, attachment, audit) as spec:
        # siGit denies CLT paths under /Library. The framework executable finds
        # libpython relative to itself, so put the runtime inside the test root.
        source = Path(sys.base_prefix).resolve()
        executable = Path(spec["command"])
        require(source.parent.parent.name == "Python3.framework"
                and executable.is_relative_to(source), "siGit requires the macOS Python framework")
        target = workspace / ".mcp-probe" / "Python3.framework" / "Versions" / source.name
        shutil.copytree(source, target, symlinks=True)
        yield dict(spec, command=str(target / executable.relative_to(source)))


def prepare_workspace(workspace):
    # A project skill symlink must not expose a source outside the native roots.
    unselected, = (workspace / ".shiju/skills").glob("unselected-probe-*")
    (workspace / NATIVE_DIR / unselected.name).symlink_to(unselected, target_is_directory=True)


class Probe(ChatProbe):
    def response(self, body):
        require(body["model"] == MODEL, "unexpected siGit model")
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if not tools:
            return {"content": "siGit fixture session"}, "stop"
        self.requests += 1
        skills = self.fixtures["skills"]
        if self.requests == 1:
            require("read_file" in tools, "siGit file callback tool missing")
            if "skill" not in tools:
                self.failure = "siGit native skill tool missing from the model catalog"
                return {"content": "SIGIT_BASELINE_DONE"}, "stop"
            catalog = tools["skill"]["description"]
            listed = re.findall(r"^- ([a-z0-9-]+): ", catalog, re.M)
            envelope = json.dumps(body)
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require(listed.count(skill["name"]) == int(visible), f"unexpected siGit {kind} catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "siGit included skill content before activation")
            require(self.fixtures["mcp"] not in envelope, "siGit included MCP data before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected siGit MCP catalog")
            return self.activate(tools, "native")

        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"]) for row in body["messages"]
                   if row["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        self.completed[step] = call_id
        if step in ("native", "selected"):
            skill = skills[step]
            require(skill["body"] in result and skill["attachment"] not in result,
                    f"unexpected siGit {step} activation")
            match = re.search(r"Its directory is (.+?) — resolve", result)
            require(match is not None, "siGit activation omitted the directory")
            directory = Path(match[1])
            require(directory.is_absolute() and directory.resolve() == skill["path"].resolve(),
                    f"siGit activated an unexpected {step} directory")
            return self.call(tools, step + "_read", "read_file",
                             {"path": str(directory / "references/value.txt")})
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and re.search(r"reject|denied|cancel", result, re.I))
                    if self.rejected else value in result, "unexpected siGit MCP continuation")
        else:
            require(skills[step.removesuffix("_read")]["attachment"] in result,
                    f"siGit {step} attachment missing")
        if self.mounted and step == "native_read":
            return self.activate(tools, "selected")
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == f"{MCP_TOOL}({{}})"
                and metadata.get("arguments") == {})
