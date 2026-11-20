"""Exercise Dirac's native skill tools and the reviewed stdio MCP bridge."""
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
from .opencode import Handler, Probe as ChatProbe

MODEL = "selected-model"
MODEL_REASONING = True
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".dirac/skills"
MCP_TOOL = "coexistence-read_probe"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Dirac platform")
    contract_path = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(contract_path.read_text())["dirac"]
    require(contract["version"] == "0.5.24", "unexpected Dirac contract version")
    source = Path(os.environ.get("ACP_DIRAC_NPM_PREFIX",
                                 ROOT / "target/acp-skill-coexistence/dirac-npm"))
    packages = json.loads((source / "package-lock.json").read_text())["packages"]
    for name, version, integrity in (
        (contract["package"], contract["version"], contract["integrity"]),
        (contract["engine"], contract["engine_version"], contract["engine_integrity"]),
    ):
        package = packages["node_modules/" + name]
        require((package["version"], package["integrity"]) == (version, integrity),
                "Dirac package lock does not match the reviewed release")
    modules = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "node_modules"
    shutil.copytree(source / "node_modules", modules, symlinks=True)
    package = modules / contract["package"]
    upstream = package / "dist/cli.mjs"
    try:
        subprocess.run([sys.executable, str(ROOT / "patches/acp-model-source/dirac/apply.py"),
                        str(upstream.parent)], env=env, check=True, capture_output=True, timeout=30)
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
        raise AssertionError("Dirac patch preparation failed") from None
    for name, expected in contract["files"].items():
        require(hashlib.sha256((package / name).read_bytes()).hexdigest() == expected,
                "Dirac artifact digest mismatch")
    entry = package / contract["bin"]
    return {"command": str(entry), "args": ["--acp"]}, {
        "version": contract["version"], "platform": "darwin-arm64",
        "upstream_sha256": hashlib.sha256(upstream.read_bytes()).hexdigest(),
        "sha256": contract["files"][contract["bin"]],
        "mcp_helper_sha256": contract["files"]["dist/session-mcp.mjs"],
    }


class Probe(ChatProbe):
    def response(self, body):
        require(body["model"] == MODEL, "unexpected Dirac model")
        tools = {tool["function"]["name"]: tool["function"] for tool in body.get("tools", [])}
        if not tools:
            return {"content": "Dirac fixture session"}, "stop"
        messages = body["messages"]
        self.requests += 1
        skills = self.fixtures["skills"]
        if self.requests == 1:
            system = "\n".join(str(row["content"]) for row in messages if row["role"] == "system")
            envelope = json.dumps(body)
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in system) == visible, f"unexpected Dirac {kind} catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Dirac included skill contents before activation")
            require(self.fixtures["mcp"] not in envelope, "Dirac included MCP value before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected Dirac MCP inventory")
            return self.activate(tools, "native")

        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"]) for row in messages if row["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        self.completed[step] = call_id
        if step in ("native", "selected"):
            skill = skills[step]
            require(skill["body"] in result and skill["attachment"] not in result,
                    f"unexpected Dirac {step} activation result")
            match = re.search(r"skill directory at: ([^\n]+)", result)
            require(match is not None, "Dirac activation omitted the skill directory")
            directory = Path(match[1].strip())
            require(directory.is_absolute() and directory.resolve() == skill["path"].resolve(),
                    f"Dirac activated an unexpected {step} directory")
            return self.call(tools, step + "_read", "read_file",
                             {"paths": [str(directory / "references/value.txt")]})
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and "Permission denied by user" in result)
                    if self.rejected else value in result, "unexpected Dirac MCP continuation")
        else:
            require(skills[step.removesuffix("_read")]["attachment"] in result,
                    f"Dirac {step} could not read its attachment")
        if self.mounted and step == "native_read":
            return self.activate(tools, "selected")
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        if "respond" in tools:
            return self.call(tools, "respond", "respond",
                             {"operation": "complete", "text": "COEXISTENCE_DONE"})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def activate(self, tools, kind):
        return self.call(tools, kind, "use_skill",
                         {"skill_name": self.fixtures["skills"][kind]["name"]})
