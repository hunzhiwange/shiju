"""Goose v1.57.0 OpenAI Chat Completions and native load_skill probe."""
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

from . import correlated_result

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
MCP_TOOL = "coexistence__read_probe"
NATIVE_DIR = ".agents/skills"


def launch(env):
    machine = platform.machine()
    linux = platform.system() == "Linux" and machine in ("x86_64", "aarch64")
    require(linux or (platform.system(), machine) == ("Darwin", "arm64"),
            "unreviewed Goose platform")
    target = f"{machine}-unknown-linux-gnu" if linux else None
    runtime_name = (f"goose-v1.57.0-model-source-v2-{target}" if linux
                    else "goose-v1.57.0-model-source-v2")
    home = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    default = home / "acp-runtime" / runtime_name / "goose"
    override = os.environ.get("ACP_COEXISTENCE_GOOSE_BIN")
    source = Path(override or default).resolve()
    require(source.is_file(), "reviewed Goose executable unavailable")
    registry = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(registry.read_text())["goose"]
    receipt = json.loads(source.with_name("build-receipt.json").read_text())
    # An explicit artifact can reproduce the pre-fix behavior with the old gateway.
    old_patch = "3a47bbbdcf861d680ff051879d81d23ef872d035a01a0549f9cedecd65b54e06"
    legacy = not linux and bool(override) and receipt.get("patch_sha256") == old_patch
    require(all(receipt.get(key) == contract[key] for key in ("version", "source_sha256"))
            and (legacy or receipt.get("patch_sha256") == contract["patch_sha256"]),
            "Goose build receipt differs from reviewed contract")
    if linux:
        require(receipt.get("target") == target, "Goose build receipt is for a different target")
    if legacy:
        runtime_name = "goose-v1.57.0-model-source-v1"
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / runtime_name
    runtime.mkdir(parents=True, mode=0o700)
    entry = runtime / "goose"
    shutil.copy2(source, entry)
    sha256 = hashlib.sha256()
    with entry.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            sha256.update(block)
    require(sha256.hexdigest() == receipt.get("sha256"), "Goose executable digest differs from build receipt")
    identity = {key: receipt[key] for key in ("version", "source_sha256", "patch_sha256", "sha256")}
    if linux:
        identity.update(target=target, entry=entry.name)
    (runtime / "build-receipt.json").write_text(json.dumps(identity))
    try:
        version = subprocess.run([str(entry), "--version"], env=env, capture_output=True, timeout=20)
    except subprocess.TimeoutExpired as error:
        raise AssertionError("Goose version probe timed out") from error
    require(version.returncode == 0 and version.stdout.strip() == b"1.57.0", "unexpected Goose version")
    spec = {"command": str(entry), "args": ["acp"], "env": {"GOOSE_DISABLE_SESSION_NAMING": "true"}}
    return spec, identity


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.calls = {}
        self.completed = {}
        self.failure = None
        self.mcp_pending = False
        self.skill_pending = None
        # Keep the historical Darwin probe ordering and extend Linux with a
        # read from the built-in developer extension.
        self.verify_tree = platform.system() == "Linux"
        self.tree_call_id = None
        self.tree_arguments = {"path": str(fixtures["skills"]["native"]["path"]), "depth": 1}

    def response(self, body):
        self.requests += 1
        require(self.failure is None, self.failure)
        require(body["model"] == MODEL, "unexpected Goose upstream model")
        messages = body["messages"]
        tools = {entry["function"]["name"]: entry["function"] for entry in body.get("tools", [])}
        if self.requests == 1:
            catalog = "\n".join(str(msg["content"]) for msg in messages if msg["role"] == "system")
            sections = re.findall(r"# Skills\n\nYou have these skills at your disposal\..*?(?=\n# |\Z)",
                                  catalog, re.S)
            names = set(re.findall(r"^- (\S+): ", "\n".join(sections), re.M))
            envelope = json.dumps(body)
            for kind, skill in self.fixtures["skills"].items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in names) == visible, f"unexpected {kind} skill catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "skill content appeared before activation")
            require("goose-doc-guide" in names, "Goose built-in skill catalog missing")
            require(self.fixtures["mcp"] not in envelope, "MCP value appeared before invocation")
            aliases = sorted(name for name in tools if "probe" in name or "coexist" in name)
            require((MCP_TOOL in tools) == self.mounted, f"unexpected MCP tool catalog {aliases}")

        results = [dict(msg, tool_use_id=msg["tool_call_id"]) for msg in messages if msg["role"] == "tool"]
        if self.verify_tree:
            if self.tree_call_id is None:
                candidates = [name for name in ("tree", "developer__tree") if name in tools]
                require(len(candidates) == 1, "Goose native developer tree is missing or ambiguous")
                response = self.call(tools, "tree", candidates[0], self.tree_arguments)
                self.tree_call_id = self.calls.pop("tree")
                return response
            correlated_result(results, self.tree_call_id, "SKILL.md")
        for kind, skill in self.fixtures["skills"].items():
            if kind == "unselected" or (kind == "selected" and not self.mounted):
                continue
            for step, name, value in ((kind, skill["name"], skill["body"]),
                                      (kind + "_read", skill["name"] + "/references/value.txt",
                                       skill["attachment"])):
                if step not in self.calls:
                    self.skill_pending = (step, name)
                    return self.call(tools, step, "load_skill", {"name": name})
                result = correlated_result(results, self.calls[step])
                content = str(result["content"])
                if value not in content:
                    known_errors = [message for message in ("Failed to read", "Not a directory",
                                    "Too many levels", "not found") if message.lower() in content.lower()]
                    raise AssertionError(f"Goose {step} result lacks marker {known_errors}")
                if step == kind and step not in self.completed:
                    self.check_activation(result, kind, skill)
                self.completed[step] = self.calls[step]
        self.skill_pending = None
        if self.mounted:
            if "mcp" not in self.calls:
                self.mcp_pending = True
                return self.call(tools, "mcp", MCP_TOOL, {})
            # Chat Completions tool messages carry text but no MCP error flag.
            result = correlated_result(results, self.calls["mcp"])
            value = str(result["content"])
            if self.rejected:
                require("declined" in value.lower() and self.fixtures["mcp"] not in value,
                        "Goose did not return the native MCP rejection")
            else:
                require(self.fixtures["mcp"] in value, "Goose MCP result lacks fixture value")
            self.completed["mcp"] = self.calls["mcp"]
            self.mcp_pending = False
        return {"content": "COEXISTENCE_DONE"}, "stop"

    @staticmethod
    def check_activation(result, kind, skill):
        content = str(result["content"])
        prefix = "Skill directory: "
        directory = next((Path(line[len(prefix):]) for line in content.splitlines()
                          if line.startswith(prefix)), None)
        require(directory is not None and directory == skill["path"].resolve(),
                f"{kind} activation did not identify the canonical fixture directory")
        require(f'load_skill(name: "{skill["name"]}/references/value.txt")' in content,
                f"{kind} skill attachment was not advertised by native activation")

    def call(self, tools, step, name, args):
        require(name in tools, f"{step} tool missing from Goose catalog")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= set(args) <= set(schema.get("properties", {})),
                f"{step} tool schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.calls[step] = call_id
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(args)}}]}, "tool_calls"

    def approves(self, metadata):
        return (self.mcp_pending and metadata.get("title") == "coexistence: read probe"
                and metadata.get("arguments") == {})

    def approves_skill(self, metadata):
        if (self.tree_call_id and self.skill_pending is None and not self.calls
                and metadata.get("arguments") == self.tree_arguments
                and str(metadata.get("title", "")).startswith(("tree · ", "developer: tree · "))):
            return "tree"
        if self.skill_pending is not None:
            step, name = self.skill_pending
            if (metadata.get("title") == f"load skill · {name}"
                    and metadata.get("arguments") == {"name": name}):
                return step
        return None

    def expected_skill_approvals(self):
        steps = [step for step in self.calls if step != "mcp"]
        return ["tree", *steps] if self.verify_tree else steps


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated model request")
            require(self.path == "/v1/chat/completions", "unexpected Goose model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Goose Chat Completions envelope")
            self.send_error(422, "probe rejected model request")
            return
        message = dict(delta, role="assistant")
        self.send_response(200)
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
                "created": 1, "model": body["model"], "choices": [
                    {"index": 0, "finish_reason": finish, "message": message}]}).encode())
