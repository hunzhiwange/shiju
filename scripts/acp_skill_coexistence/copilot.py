"""Probe Copilot 1.0.105 native skills and its ACP SSE MCP transport."""

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
MODEL_PROTOCOL = API_PATH + "/chat/completions"
TRANSPORT = "sse"
NATIVE_DIR = ".github/skills"
REVISION = "copilot-1.0.105-model-source-v2"
MCP_FUNCTION = "coexistence-read_probe"
DISABLED_SKILL = "coexistence-disabled-native"
IGNORED_SKILL = "coexistence-ignored-native"


def launch(env):
    system, machine = platform.system(), platform.machine()
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json")
                          .read_text())["copilot"]
    require(contract["version"] == "1.0.105", "unexpected Copilot contract version")
    revision = REVISION
    files = contract["files"]
    artifact = {}
    if system == "Linux":
        require(os.geteuid() != 0 and platform.libc_ver()[0] == "glibc",
                "Copilot Linux probe requires a non-root GNU environment")
        linux = json.loads((ROOT / "patches/acp-model-source/copilot/linux-artifacts.json").read_text())
        require(machine in linux["targets"] and linux["version"] == contract["version"]
                and linux["bin"] == contract["bin"], "unreviewed Copilot Linux platform")
        target_contract = linux["targets"][machine]
        revision = linux["revision"]
        files = linux["files"] | target_contract["files"]
        artifact = {"platform_package": target_contract["package"], "entry": linux["bin"],
                    "model_protocol": MODEL_PROTOCOL}
    else:
        require((system, machine) == ("Darwin", "arm64"), "unreviewed Copilot platform")
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    runtime = Path(os.environ.get("COPILOT_COEXISTENCE_RUNTIME", base / "acp-runtime" / revision))
    if system == "Linux":
        native = runtime / "node_modules" / artifact["platform_package"] / "copilot"
        require(artifact_digest(native) == target_contract["sea_sha256"],
                "Copilot Linux supplier SEA digest differs from contract")
        artifact["sea_sha256"] = target_contract["sea_sha256"]
    source = runtime / "node_modules/@github/copilot"
    require(source.is_dir(), "reviewed Copilot runtime is unavailable")
    staged = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision
    target = staged / "node_modules"
    package = target / "@github/copilot"
    entry = target / ".bin/copilot"
    if system == "Linux":
        shutil.copytree(runtime, staged, symlinks=True)
        require(entry.is_symlink() and entry.resolve(strict=True) == package / contract["bin"],
                "Copilot Linux executable does not resolve to the staged fixed entry")
    else:
        shutil.copytree(source, package)
        entry.parent.mkdir()
        entry.symlink_to("../@github/copilot/" + contract["bin"])
    for name, expected in files.items():
        path = package / name
        require(path.resolve(strict=True) == path and artifact_digest(path) == expected,
                "Copilot artifact digest differs from contract")
    version = subprocess.run([str(entry), "--no-auto-update", "--version"], env=env, cwd=target,
                             capture_output=True, timeout=15, check=False)
    require(version.returncode == 0 and b"1.0.105" in version.stdout,
            "Copilot entry did not report the pinned version")
    return {"command": str(entry), "args": ["--acp", "--stdio"],
            "env": {"SHIJU_COPILOT_PROJECT_SKILLS": str(Path(env["HOME"]).parent / "workspace")}}, {
        **artifact, "version": contract["version"], "revision": revision, "sha256": files,
    }


def artifact_digest(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def prepare_workspace(workspace):
    settings = workspace / ".github/copilot/settings.json"
    settings.parent.mkdir(parents=True)
    for directory, name in ((NATIVE_DIR, DISABLED_SKILL), (".agents/skills", IGNORED_SKILL)):
        skill = workspace / directory / name
        skill.mkdir(parents=True)
        (skill / "SKILL.md").write_text(
            f"---\nname: {name}\ndescription: Excluded native fixture\n---\nRead a fixture value.\n")
    settings.write_text(json.dumps({"skillDirectories": [NATIVE_DIR, ".agents/skills"],
                                    "disabledSkills": [DISABLED_SKILL],
                                    "ignoredSkillsLocations": [".agents/skills"]}))


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.calls = {}
        self.completed = {}
        self.failure = None
        self.pending_mcp = False

    def response(self, body):
        require(body["model"] == MODEL, "unexpected Copilot model")
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        messages = body["messages"]
        self.requests += 1
        has_mcp = MCP_FUNCTION in tools
        if self.requests == 1:
            catalog = json.dumps([m for m in messages if m["role"] in ("system", "developer")])
            require(DISABLED_SKILL not in catalog and IGNORED_SKILL not in catalog,
                    "Copilot exposed a disabled or ignored native skill")
            request = json.dumps(body)
            for kind, skill in self.fixtures["skills"].items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog) == visible,
                        f"unexpected Copilot {kind} skill catalogue")
                require(skill["body"] not in request and skill["attachment"] not in request,
                        "Copilot skill content leaked before activation")
            require(self.fixtures["mcp"] not in request, "MCP value leaked before invocation")
            require(has_mcp == self.mounted, "unexpected Copilot SSE MCP catalogue")
        for kind in ("native", "selected") if self.mounted else ("native",):
            skill = self.fixtures["skills"][kind]
            if kind not in self.calls:
                return self.call(tools, kind, "skill", {"skill": skill["name"]})
            self.result(messages, kind)
            expanded = [row["content"] for row in messages
                        if isinstance(row.get("content"), str) and skill["body"] in row["content"]]
            require(expanded, "Copilot skill body was not expanded")
            read = kind + "_read"
            if read not in self.calls:
                match = re.search(r"Base directory for this skill: ([^\n]+)", "\n".join(expanded))
                require(match is not None, "Copilot activation omitted the base directory")
                base = Path(match[1].strip())
                require(base.is_absolute() and base.name == skill["name"] and
                        (base / "SKILL.md").resolve() == (skill["path"] / "SKILL.md").resolve(),
                        "Copilot activated an unexpected skill path")
                if kind == "native":
                    require(base == skill["path"], "Copilot bypassed the native directory")
                else:
                    require(base.parent.parent.name.startswith("shiju-session-skills-"),
                            "Copilot bypassed the session skill view")
                return self.call(tools, read, "view", {"path": str(base / "references/value.txt")})
            require(skill["attachment"] in self.result(messages, read),
                    f"Copilot {kind} attachment was not read")
        if self.mounted:
            if "mcp" not in self.calls:
                require(has_mcp, "Copilot SSE MCP tool disappeared")
                self.pending_mcp = True
                return self.call(tools, "mcp", MCP_FUNCTION, {})
            result = self.result(messages, "mcp")
            if self.rejected:
                require(re.search(r"reject|denied|cancel", result, re.I)
                        and self.fixtures["mcp"] not in result, "Copilot MCP refusal missing")
            else:
                require(self.fixtures["mcp"] in result, "Copilot MCP value was not read")
        self.pending_mcp = False
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def call(self, tools, kind, name, arguments):
        require(name in tools, f"Copilot {kind} tool missing")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= set(arguments) <= set(schema.get("properties", {})),
                f"Copilot {kind} schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.calls[kind] = call_id
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def result(self, messages, kind):
        call_id = self.calls[kind]
        results = [row["content"] for row in messages
                   if row.get("role") == "tool" and row.get("tool_call_id") == call_id]
        require(len(results) == 1, f"Copilot {kind} result missing or ambiguous")
        self.completed[kind] = call_id
        return results[0] if isinstance(results[0], str) else json.dumps(results[0])

    def observe_terminal(self, events):
        if not self.rejected or not self.pending_mcp or "mcp" in self.completed:
            return False
        call_id = self.calls["mcp"]
        failures = [event for event in events if event.get("type") == "chat.tool"
                    and event.get("call_id") == call_id and event.get("kind") == "complete"]
        if len(failures) != 1:
            return False
        failure = failures[0]
        payload = failure["payload"]
        if (failure.get("status") == "failed" and payload.get("title") == MCP_FUNCTION
                and payload.get("error") == "The user rejected this tool call."):
            self.completed["mcp"] = call_id
            return True
        return False

    def approves(self, metadata):
        return (self.pending_mcp and metadata.get("arguments") == {}
                and metadata.get("title") == "read_probe")


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Copilot model request")
            require(self.path == MODEL_PROTOCOL, "unexpected Copilot model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Copilot model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Copilot Chat Completions envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        if body.get("stream"):
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            self.chunk(body["model"], dict(delta, role="assistant"), None)
            self.chunk(body["model"], {}, finish)
            self.wfile.write(b"data: [DONE]\n\n")
        else:
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"id": "chatcmpl-fixture", "object": "chat.completion",
                "model": body["model"], "choices": [{"index": 0, "finish_reason": finish,
                "message": dict(delta, role="assistant")}]}).encode())
