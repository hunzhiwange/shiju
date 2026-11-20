"""Probe the reviewed Amp ThreadWorker via its Anthropic Messages relay."""

import hashlib
import json
import os
from pathlib import Path
import platform
import secrets
import shutil
import subprocess
import sys
import tempfile

from verify_acp_integration import require
from verify_acp_model_source import ROOT

from . import correlated_result
from .claude import Handler as Handler

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "anthropic"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".agents/skills"
MCP_TOOL = "mcp__coexistence__read_probe"
MANAGED = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"
VERSION = "0.12.0"
ENGINE = "0.0.1778732631-gc5875c"
REVISION = f"amp-acp-{VERSION}-model-source-v5"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Amp platform")
    manifest = MANAGED / "contracts.json"
    contract = json.loads(manifest.read_text())["amp-acp"]
    require((contract["version"], contract["engine_version"]) == (VERSION, ENGINE),
            "unexpected Amp contract version")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    runtime.parent.mkdir(mode=0o700)
    source = os.environ.get("SHIJU_AMP_PROBE_RUNTIME")
    if source:
        shutil.copytree(Path(source).resolve(strict=True), runtime, symlinks=True)
    else:
        with tempfile.TemporaryDirectory(dir=env["TMPDIR"]) as directory:
            assets = Path(directory)
            shutil.copy2(manifest, assets)
            shutil.copytree(ROOT / "patches/acp-model-source/amp-acp", assets / "amp-acp")
            try:
                install = subprocess.run(
                    [sys.executable, str(MANAGED / "install.py"), directory, str(runtime), "amp-acp"],
                    env=env, capture_output=True, timeout=180,
                )
            except subprocess.TimeoutExpired as error:
                raise AssertionError("Amp reviewed runtime installation timed out") from error
            require(install.returncode == 0, "Amp reviewed runtime installation failed")

    modules = runtime / "node_modules"
    package = modules / "amp-acp"
    for name, version in (("amp-acp", VERSION), ("@sourcegraph/amp", ENGINE)):
        meta = json.loads((modules / name / "package.json").read_text())
        require((meta["name"], meta["version"]) == (name, version),
                "unexpected Amp package identity")
    hashes = {}
    for name, digest in contract["files"].items():
        file = (package / name).resolve(strict=True)
        require(file.is_relative_to(modules), "redirected Amp artifact")
        actual = hashlib.sha256(file.read_bytes()).hexdigest()
        require(actual == digest, "Amp reviewed artifact digest mismatch")
        hashes[name] = actual
    command = modules / ".bin/amp-acp"
    require(command.resolve(strict=True) == package / contract["bin"],
            "Amp launch entry does not resolve to the reviewed adapter")
    node = subprocess.check_output(["node", "--version"], env=env, text=True, timeout=10).strip()
    return {"command": str(command), "args": []}, {
        "version": VERSION, "engine": ENGINE, "revision": REVISION, "node": node, "sha256": hashes,
    }


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.pending = None
        self.failure = None

    def response(self, body):
        require(body["model"] == MODEL, "unexpected Amp upstream model")
        if not body.get("stream"):
            return {"content": "Fixture session"}
        self.requests += 1
        tools = {tool["name"]: tool for tool in body.get("tools", [])}
        skills = self.fixtures["skills"]
        require((MCP_TOOL in tools) == self.mounted, "unexpected Amp stdio MCP catalog")
        if self.pending is None:
            catalog = json.dumps([body.get("system"), tools, body["messages"]])
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog) == visible, f"unexpected Amp {kind} skill catalog")
                require(skill["body"] not in catalog and skill["attachment"] not in catalog,
                        "Amp skill content appeared before activation")
            require(self.fixtures["mcp"] not in catalog, "Amp MCP value appeared before invocation")
            return self.call(tools, "native", "skill", {"name": skills["native"]["name"]})

        step, _, _, call_id = self.pending
        results = [block for row in body["messages"] if row["role"] == "user"
                   and isinstance(row["content"], list)
                   for block in row["content"] if block["type"] == "tool_result"]
        kind = step.removesuffix("_read")
        skill = skills.get(kind)
        if step == "mcp":
            expected = "Amp host tool rejected" if self.rejected else self.fixtures["mcp"]
        else:
            expected = skill["attachment" if step.endswith("_read") else "body"]
        result = correlated_result(results, call_id, expected, rejected=step == "mcp" and self.rejected)
        if step == "mcp" and self.rejected:
            require(self.fixtures["mcp"] not in str(result["content"]),
                    "rejected Amp MCP call returned the fixture value")
        self.completed[step] = call_id
        if step in ("native", "selected"):
            content = result["content"]
            if isinstance(content, list):
                content = "\n".join(block["text"] for block in content if block["type"] == "text")
            require(content.startswith("<content>") and content.endswith("</content>"),
                    "Amp activation lacks its native content envelope")
            blocks = json.loads(content.removeprefix("<content>").removesuffix("</content>"))
            content = "\n".join(block["text"] for block in blocks if block["type"] == "text")
            prefix = "Base directory for this skill: "
            base = next((Path(line[len(prefix):]) for line in content.splitlines()
                         if line.startswith(prefix)), None)
            require(base is not None and base.is_absolute() and base.name == skill["name"],
                    "Amp activation omitted its skill directory")
            if step == "native":
                require(base.resolve() == skill["path"].resolve(), "unexpected Amp native skill path")
            else:
                require(any(part.startswith("shiju-session-skills-") for part in base.parts)
                        and base.resolve() == skill["path"].resolve(),
                        "Amp selected skill bypassed the session view")
            return self.call(tools, step + "_read", "Read", {"path": str(base / "references/value.txt")})
        if step == "native_read" and self.mounted:
            return self.call(tools, "selected", "skill", {"name": skills["selected"]["name"]})
        if step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        return {"content": "COEXISTENCE_DONE"}

    def call(self, tools, step, name, arguments):
        require(name in tools, f"Amp {step} tool missing")
        schema = tools[name]["input_schema"]
        require(set(schema.get("required", [])) <= arguments.keys() <= schema.get("properties", {}).keys(),
                f"unexpected Amp {step} tool schema")
        call_id = "toolu_" + secrets.token_hex(12)
        self.pending = (step, name, arguments, call_id)
        return {"tool_calls": [{"id": call_id, "function": {
            "name": name, "arguments": json.dumps(arguments)}}]}

    def approves(self, metadata):
        return self.matches(metadata, "mcp")

    def approves_skill(self, metadata):
        for step in ("native", "native_read", "selected", "selected_read"):
            if self.matches(metadata, step):
                return step

    def matches(self, metadata, step):
        if self.pending is None:
            return False
        actual, name, args, _ = self.pending
        return (actual == step and metadata.get("title") == name
                and (metadata.get("arguments") or {}) == args)
