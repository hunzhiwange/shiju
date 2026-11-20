"""Snow 0.9.14 native skill-execute and stdio MCP via Chat Completions."""
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".snow/skills"
SKILL_TOOL = "skill-execute"
READ_TOOL = "filesystem-read"
MCP_TOOL = "coexistence-read_probe"
DISABLED_SKILL = "snow-disabled-probe"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Snow platform")
    require(shutil.which("node", path=env["PATH"]), "Node is unavailable")
    contracts = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(contracts.read_text())["snow"]
    require(contract["version"] == "0.9.14", "Snow version differs from pin")
    revision = "snow-0.9.14-model-source-v4"
    config = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = Path(os.environ.get("SNOW_COEXISTENCE_RUNTIME", config / "acp-runtime" / revision))
    require((source / "node_modules" / contract["package"]).is_dir(),
            "reviewed Snow runtime unavailable")
    private = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision
    shutil.copytree(source, private, symlinks=True)
    package = private / "node_modules" / contract["package"]
    metadata = json.loads((package / "package.json").read_text())
    require((metadata["name"], metadata["version"]) == (contract["package"], contract["version"]),
            "Snow artifact identity differs from contract")
    for name, digest in contract["files"].items():
        require(hashlib.sha256((package / name).read_bytes()).hexdigest() == digest,
                "Snow artifact digest differs from contract")
    entry = package / contract["bin"]
    version = subprocess.run([str(entry), "--version"], cwd=private, env=env,
                             capture_output=True, timeout=15, check=False)
    require(version.returncode == 0 and b"0.9.14" in version.stdout,
            "Snow entry did not report the pinned version")
    return {"command": str(entry), "args": ["--acp"], "env": {}}, {
        "version": contract["version"], "sha256": contract["files"], "revision": revision}


def prepare_workspace(workspace):
    disabled = workspace / NATIVE_DIR / DISABLED_SKILL
    disabled.mkdir()
    (disabled / "SKILL.md").write_text(
        f"---\nname: {DISABLED_SKILL}\ndescription: Disabled native fixture\n---\nDisabled fixture\n")
    (workspace / ".snow/settings.json").write_text(json.dumps({"disabledSkills": [DISABLED_SKILL]}))


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
        require(body["model"] == MODEL and body.get("stream"), "unexpected Snow model request")
        tools = {row["function"]["name"]: row["function"] for row in body["tools"]}
        messages = body["messages"]
        self.requests += 1
        description = tools.get(SKILL_TOOL, {}).get("description", "")
        names = set(re.findall(r"<skill>\s*<name>\s*([^<\s]+)\s*</name>", description))
        require(DISABLED_SKILL not in names, "disabled Snow skill was discoverable")
        for kind, skill in self.fixtures["skills"].items():
            expected = kind == "native" or (kind == "selected" and self.mounted)
            require((skill["name"] in names) == expected, f"unexpected {kind} skill catalog")
        require((MCP_TOOL in tools) == self.mounted, "unexpected Snow MCP catalog")
        if self.requests == 1:
            wire = json.dumps(body)
            for skill in self.fixtures["skills"].values():
                require(skill["body"] not in wire and skill["attachment"] not in wire,
                        "skill content appeared before activation")
            require(self.fixtures["mcp"] not in wire, "MCP value appeared before invocation")
            return self.activate(tools, "native")
        kind, _, call_id = self.pending
        results = [msg["content"] for msg in messages if msg.get("role") == "tool"
                   and msg.get("tool_call_id") == call_id]
        require(len(results) == 1 and isinstance(results[0], str),
                f"missing or ambiguous {kind} continuation")
        result = results[0]
        self.completed[kind] = call_id
        if kind in ("native", "selected"):
            skill = self.fixtures["skills"][kind]
            require(skill["body"] in result and skill["attachment"] not in result,
                    f"unexpected {kind} activation content")
            match = re.search(r"^Absolute Path: (.+)$", result, re.M)
            require(match is not None, "native activation omitted its directory")
            base = Path(match[1])
            require(base.is_absolute() and base.name == skill["name"]
                    and base.resolve() == skill["path"].resolve(), "unexpected skill directory")
            if kind == "native":
                require(base == skill["path"], "native skill bypassed its project directory")
            else:
                require(base.parent.name == "skills"
                        and base.parent.parent.name.startswith("shiju-session-skills-"),
                        "selected skill bypassed the session view")
            return self.call(tools, kind + "_read", READ_TOOL,
                             {"filePath": str(base / "references/value.txt")})
        if kind == "mcp":
            value = self.fixtures["mcp"]
            require(("Permission denied by user" in result and value not in result)
                    if self.rejected else value in result, "unexpected MCP continuation")
        else:
            skill = self.fixtures["skills"][kind.removesuffix("_read")]
            require(skill["attachment"] in result, f"{kind} did not return the attachment")
            if self.mounted:
                if kind == "native_read":
                    return self.activate(tools, "selected")
                return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def activate(self, tools, kind):
        return self.call(tools, kind, SKILL_TOOL, {"skill": self.fixtures["skills"][kind]["name"]})

    def call(self, tools, kind, name, arguments):
        require(name in tools, f"{kind} tool missing from Snow catalog")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", ())) <= set(arguments) <= set(schema.get("properties", {})),
                f"{kind} tool schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.pending = (kind, name, call_id)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == MCP_TOOL and metadata.get("arguments") == {})

    def approves_skill(self, metadata):
        if self.pending is None:
            return None
        kind, name, _ = self.pending
        # Snow's native ACP permission request reports the tool title without rawInput.
        if kind != "mcp" and metadata.get("title") == name and metadata.get("arguments") is None:
            return kind
        return None

    def expected_skill_approvals(self):
        return ["native", "native_read", "selected", "selected_read"] if self.mounted else [
            "native", "native_read"]


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Snow model request")
            require(self.path == "/v1/chat/completions", "unexpected Snow model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid model request size")
            delta, finish = probe.response(json.loads(self.rfile.read(size)))
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Snow Chat Completions envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.chunk(MODEL, dict(delta, role="assistant"), None)
        self.chunk(MODEL, {}, finish)
        self.wfile.write(b"data: [DONE]\n\n")
