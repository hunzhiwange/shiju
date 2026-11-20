"""Exercise Nova's native Anthropic tool loop and owner-approved stdio MCP."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil

from verify_acp_integration import require
from verify_acp_model_source import ROOT

from . import correlated_result
from .claude import Handler, Probe as MessagesProbe
from .cortex_code import mcp_server

MODEL = "selected-model"
ADAPTER = "anthropic"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = "../home/.nova/skills"
MCP_TOOL = "mcp_shiju_coexistence__read_probe"
DISABLED_SKILL = "disabled-nova-coexistence-fixture"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Nova platform")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/"
                           "contracts.json").read_text())["nova"]
    require(contract["version"] == "1.1.65", "unexpected Nova release")
    prefix = os.environ.get("ACP_NOVA_REVIEWED_PREFIX")
    consent = os.environ.get("ACP_NOVA_TEST_CONSENT")
    require(prefix and consent, "Nova requires a reviewed install and an authorized test consent file")
    modules = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "node_modules"
    shutil.copytree(Path(prefix) / "node_modules", modules, symlinks=True)
    package = modules / contract["package"]
    require(json.loads((package / "package.json").read_text())["version"] == contract["version"],
            "Nova package version mismatch")
    for path, digest in contract["files"].items():
        require(hashlib.sha256((package / path).read_bytes()).hexdigest() == digest,
                "Nova managed artifact digest mismatch")
    state = Path(env["HOME"]) / ".nova"
    state.mkdir(mode=0o700)
    target = state / "consent.json"
    target.touch(mode=0o600)
    target.write_bytes(Path(consent).read_bytes())
    return {"command": str(package / contract["bin"]), "args": ["acp"]}, {
        "version": contract["version"], "platform": "darwin-arm64", "sha256": contract["files"],
    }


def prepare_workspace(workspace):
    disabled = workspace / NATIVE_DIR / DISABLED_SKILL
    disabled.mkdir()
    (disabled / "SKILL.md").write_text(
        f"---\nname: {DISABLED_SKILL}\ndescription: Disabled fixture\n"
        "disable-model-invocation: true\n---\nKeep this skill disabled.\n")


class Probe(MessagesProbe):
    def response(self, body):
        require(body["model"] == MODEL, "unexpected Nova model")
        tools = {tool["name"]: tool for tool in body.get("tools", [])}
        messages = body["messages"]
        if not tools:
            return {"content": "Nova fixture session"}
        self.requests += 1
        skills = self.fixtures["skills"]
        if self.requests == 1:
            envelope = json.dumps(body)
            require(DISABLED_SKILL not in envelope, "Nova exposed a disabled native skill")
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in envelope) == visible, f"unexpected Nova {kind} catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Nova included skill contents before activation")
            require(self.fixtures["mcp"] not in envelope, "Nova included MCP value before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected Nova MCP inventory")

        results = self.results(messages)
        if "invoke_skill" not in tools:
            require("capabilities" not in self.calls, "Nova did not activate invoke_skill")
            return self.call(tools, "capabilities", "tool_capabilities",
                             {"action": "search", "query": "invoke_skill", "activate": True})
        if "capabilities" in self.calls:
            correlated_result(results, self.calls["capabilities"], '"active":true')
        for kind in ("native", "selected") if self.mounted else ("native",):
            skill = skills[kind]
            if kind not in self.calls:
                return self.call(tools, kind, "invoke_skill", {"skill_name": skill["name"]})
            activation = correlated_result(results, self.calls[kind], skill["body"])
            require(skill["attachment"] not in str(activation), "Nova inlined a skill attachment")
            self.completed[kind] = self.calls[kind]
            read = kind + "_read"
            if read not in self.calls:
                directory = skill["path"].resolve()
                if kind == "selected":
                    directory = Path(json.loads(activation["content"])["directory"])
                    require(directory.is_absolute() and directory.parent.name == "skills" and
                            directory.parent.parent.name.startswith("shiju-session-skills-") and
                            directory.resolve() == skill["path"].resolve(),
                            "Nova did not activate the selected skill from its private view")
                return self.call(tools, read, "read_file",
                                 {"path": str(directory / "references/value.txt")})
            correlated_result(results, self.calls[read], skill["attachment"])
            self.completed[read] = self.calls[read]
        if self.mounted:
            if "mcp" not in self.calls:
                result = self.call(tools, "mcp", MCP_TOOL, {})
                self.mcp_pending = self.calls["mcp"]
                return result
            result = correlated_result(results, self.calls["mcp"], rejected=self.rejected)
            value = self.fixtures["mcp"]
            require((value not in str(result) and "denied or cancelled" in str(result))
                    if self.rejected else value in str(result), "unexpected Nova MCP result")
            self.completed["mcp"] = self.calls["mcp"]
            self.mcp_pending = None
        return {"content": "COEXISTENCE_DONE"}

    def approves(self, metadata):
        return (self.mcp_pending is not None and metadata.get("title") == MCP_TOOL
                and metadata.get("arguments") in ({}, None))
