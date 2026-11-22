"""Probe the reviewed DimCode native skill catalog and Streamable HTTP MCP."""
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

from acp_mount_integration import McpHandler
from verify_acp_integration import require
from verify_acp_model_source import ROOT

from . import correlated_result
from .opencode import Handler, Probe as OpenCodeProbe

MODEL = "selected-model"
# The native client requests this budget for tool turns.
MODEL_OUTPUT_LIMIT = 16_384
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "http"
NATIVE_DIR = ".agents/skills"
REVISION = "dimcode-0.5.28-model-source-v3"
MCP_TOOL = "coexistence__read_probe"


def launch(env):
    system, arch = platform.system(), platform.machine()
    if system == "Linux":
        require(arch in ("x86_64", "aarch64") and os.confstr("CS_GNU_LIBC_VERSION"),
                "unreviewed DimCode Linux target")
        contract = json.loads((ROOT / "patches/acp-model-source/dimcode/"
                               "linux-artifacts.json").read_text())
        expected = contract["targets"][arch]["managed_sha256"]
    else:
        require((system, arch) == ("Darwin", "arm64"), "unreviewed DimCode platform")
        contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/"
                               "managed_runtime/contracts.json").read_text())["dimcode"]
        expected = contract["files"][contract["bin"]]
    require(contract["version"] == "0.5.28", "unexpected DimCode contract version")
    relative = Path("acp-runtime") / REVISION / contract["bin"]
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = Path(os.environ.get("ACP_DIMCODE_REVIEWED_BIN", base / relative))
    require(source.is_file(), "reviewed DimCode executable unavailable")
    entry = Path(env["VIBEWINDOW_CONFIG_DIR"]) / relative
    entry.parent.mkdir(parents=True)
    shutil.copy2(source, entry)
    sha256 = hashlib.sha256()
    with entry.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            sha256.update(block)
    digest = sha256.hexdigest()
    require(digest == expected, "DimCode executable digest mismatch")
    if system == "Darwin":
        signature = subprocess.run(["/usr/bin/codesign", "--verify", "--strict", str(entry)],
                                   env=env, capture_output=True, timeout=15)
        require(signature.returncode == 0, "DimCode signature verification failed")
    version = subprocess.run([str(entry), "--version"], env=env, capture_output=True, timeout=15)
    require(version.returncode == 0 and version.stdout.strip() == b"dimcode 0.5.28",
            "unexpected DimCode executable version")
    return {"command": str(entry), "args": ["acp"]}, {
        "platform": f"{system.lower()}-{arch}", "version": contract["version"],
        "revision": REVISION, "entry": contract["bin"], "sha256": digest,
        "model_endpoint": "/v1/chat/completions",
    }


class HttpMcpHandler(McpHandler):
    def do_POST(self):
        if self.path != "/mcp" or self.headers.get("Authorization") != "Bearer " + self.server.key:
            self.close_connection = True
            self.reply(401)
            return
        super().do_POST()


@contextmanager
def mcp_server(_workspace, attachment, audit):
    from verify_acp_skill_coexistence import model_server

    key = secrets.token_hex(24)
    with model_server(HttpMcpHandler, key) as server:
        server.attachment = attachment
        server.audit = audit
        yield {"name": "coexistence", "type": "http",
               "url": f"http://127.0.0.1:{server.server_port}/mcp",
               "headers": [{"name": "Authorization", "value": "Bearer " + key}]}


class Probe(OpenCodeProbe):
    def response(self, body):
        require(body["model"] == MODEL, "unexpected DimCode model")
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if not tools:
            return {"content": "DimCode fixture session"}, "stop"
        messages = body["messages"]
        self.requests += 1
        skills = self.fixtures["skills"]
        if self.requests == 1:
            catalog = "\n".join(str(row["content"]) for row in messages if row["role"] == "system")
            envelope = json.dumps(body)
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                listed = re.search(rf"^- {re.escape(skill['name'])}(?::|$)", catalog, re.M)
                require(bool(listed) == visible, f"unexpected DimCode {kind} skill catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "DimCode inlined skill content before activation")
            require(self.fixtures["mcp"] not in envelope, "DimCode MCP fixture value leaked")
            require((MCP_TOOL in tools) == self.mounted, "unexpected DimCode HTTP MCP catalog")
            return self.activate(tools, "native")

        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"]) for row in messages if row["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        if step in ("native", "selected"):
            require("Tool call denied: source: default" not in result,
                    f"DimCode read-only permission denied {step} skill activation before host approval")
            skill = skills[step]
            require(skill["body"] in result and skill["attachment"] not in result,
                    f"unexpected DimCode {step} activation result")
            # Tool results may be JSON-wrapped by the model adapter.
            match = re.search(r"File: ([^\n]+/SKILL\.md)", result.replace("\\n", "\n"))
            require(match is not None, "DimCode activation omitted SKILL.md path")
            path = Path(match[1])
            require(path.is_absolute() and path.resolve() == skill["path"].resolve() / "SKILL.md",
                    f"unexpected DimCode {step} activation path")
            if step == "selected":
                require(path.parent.is_symlink() and path.parents[1].name == "skills"
                        and path.parents[2].name.startswith("shiju-session-skills-"),
                        "DimCode selected skill bypassed the session view")
            self.completed[step] = call_id
            return self.call(tools, step + "_read", "read",
                             {"path": str(path.parent / "references/value.txt")})
        if step == "mcp":
            value = self.fixtures["mcp"]
            if self.rejected:
                require(value not in result and re.search(r"reject|denied|cancel", result, re.I),
                        "unexpected DimCode MCP rejection result")
            else:
                require("Tool call denied: source: default" not in result,
                        "DimCode read-only permission denied HTTP MCP before host approval")
                require(value in result, "DimCode MCP value missing from continuation")
        else:
            require(skills[step.removesuffix("_read")]["attachment"] in result,
                    f"DimCode {step} could not read its attachment")
        self.completed[step] = call_id
        if self.mounted and step == "native_read":
            return self.activate(tools, "selected")
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def activate(self, tools, kind):
        return self.call(tools, kind, "skill", {"skill_name": self.fixtures["skills"][kind]["name"]})

    def observe_terminal(self, _events):
        return False
