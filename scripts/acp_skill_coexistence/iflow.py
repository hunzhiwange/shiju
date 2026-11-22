"""Verify iFlow 0.5.19's native Skill, file reader and stdio MCP tools."""

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
NATIVE_DIR = ".iflow/skills"
MCP_TOOL = "read_probe"


def launch(env):
    if platform.system() == "Linux":
        return launch_linux(env)
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed iFlow platform")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json")
                          .read_text())["iflow"]
    require(contract["version"] == "0.5.19", "unexpected iFlow contract version")
    original = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    for revision in ("iflow-0.5.19-model-source-v2", "iflow-0.5.19-model-source-v1"):
        source = original / "acp-runtime" / revision / "node_modules"
        package = source / "@iflow-ai/iflow-cli"
        if all((package / name).is_file()
               and hashlib.sha256((package / name).read_bytes()).hexdigest() == digest
               for name, digest in contract["files"].items()):
            break
    else:
        raise AssertionError("reviewed iFlow runtime is unavailable")
    target = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision / "node_modules"
    shutil.copytree(source, target, symlinks=True)
    command = target / ".bin/iflow"
    require(command.resolve() == target / "@iflow-ai/iflow-cli" / contract["bin"],
            "unexpected iFlow entrypoint")
    version = subprocess.run([str(command), "--version"], cwd=target, env=env,
                             capture_output=True, timeout=15, check=False)
    require(version.returncode == 0 and version.stdout.strip() == b"0.5.19",
            "iFlow entry did not report the pinned version")
    return {"command": str(command), "args": ["--experimental-acp"]}, {
        "version": contract["version"], "revision": revision, "sha256": contract["files"],
    }


def launch_linux(env):
    manifest = json.loads((ROOT / "patches/acp-model-source/iflow/linux-artifacts.json").read_text())
    target = manifest["targets"].get(platform.machine())
    require(target is not None, "unreviewed iFlow Linux architecture")
    source = os.environ.get("IFLOW_COEXISTENCE_RUNTIME")
    require(source, "set IFLOW_COEXISTENCE_RUNTIME to the reviewed Linux installation")

    revision = f"iflow-{manifest['version']}-model-source-v2"
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision
    shutil.copytree(Path(source).resolve(strict=True), runtime, symlinks=True)
    modules = runtime / "node_modules"
    package = modules / manifest["package"]
    entry = package / manifest["bin"]
    command = modules / ".bin/iflow"

    def digest(path):
        require(path.resolve(strict=True) == path, "redirected iFlow artifact")
        return hashlib.sha256(path.read_bytes()).hexdigest()

    lock_path = runtime / "package-lock.json"
    require(digest(lock_path) == manifest["lock_sha256"], "unreviewed iFlow dependency lock")
    lock = json.loads(lock_path.read_text())["packages"]
    integrity = lock["node_modules/" + manifest["package"]]["integrity"]
    require(integrity == manifest["integrity"], "iFlow npm archive integrity mismatch")
    observed = {name: digest(package / name) for name in manifest["patched_files"]}
    require(observed == manifest["patched_files"], "iFlow managed bundle mismatch")

    ripgrep = package / target["ripgrep"]
    require(os.access(ripgrep, os.X_OK), "iFlow bundled ripgrep is not executable")
    ripgrep_digest = digest(ripgrep)
    pty_digest = digest(modules / target["pty_package"] / "pty.node")
    require(ripgrep_digest == target["ripgrep_sha256"] and pty_digest == target["pty_sha256"],
            "iFlow Linux native dependency mismatch")
    require(command.resolve(strict=True) == entry, "unexpected iFlow entrypoint")
    version = subprocess.run([str(command), "--version"], cwd=runtime, env=env,
                             capture_output=True, timeout=15, check=False)
    require(version.returncode == 0 and version.stdout.strip().decode() == manifest["version"],
            "iFlow entry did not report the pinned version")
    args = ["--experimental-acp"]
    return {"command": str(command), "args": args}, {
        "version": version.stdout.strip().decode(), "revision": revision,
        "entry": manifest["bin"], "args": args, "sha256": observed, "integrity": integrity,
        "ripgrep_sha256": ripgrep_digest, "pty_sha256": pty_digest,
    }


def message_text(content):
    if isinstance(content, list):
        return "".join(part["text"] for part in content if part.get("type") == "text")
    return content if isinstance(content, str) else json.dumps(content)


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.calls = {}
        self.completed = {}
        self.failure = None
        self.pending = None

    def response(self, body):
        require(body["model"] == MODEL, "unexpected iFlow model")
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        self.requests += 1
        messages = body["messages"]
        if self.requests == 1:
            catalog = tools.get("Skill", {}).get("description", "")
            sections = re.findall(r"<available_skills>(.*?)</available_skills>", catalog, re.S)
            names = set(re.findall(r"<name>\s*([^<\s]+)\s*</name>", "".join(sections)))
            payload = json.dumps(body)
            for kind, skill in self.fixtures["skills"].items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in names) == visible, f"unexpected iFlow {kind} catalog")
                require(skill["body"] not in payload and skill["attachment"] not in payload,
                        "iFlow skill content leaked before activation")
            require(self.fixtures["mcp"] not in payload, "MCP value leaked before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected iFlow MCP catalog")

        for kind in ("native", "selected") if self.mounted else ("native",):
            skill = self.fixtures["skills"][kind]
            if kind not in self.calls:
                return self.call(tools, kind, "Skill", {"skill": skill["name"]})
            result, index = self.result(messages, kind)
            require("Launching skill: " + skill["name"] in result, "incorrect iFlow skill activation")
            read = kind + "_read"
            if read not in self.calls:
                later_users = (message_text(row["content"]) for row in messages[index + 1:]
                               if row["role"] == "user")
                expanded = [content for content in later_users if skill["body"] in content]
                require(len(expanded) == 1, "iFlow skill body did not reach model")
                match = re.search(r"Base directory for this skill:\s*([^\n]+)", expanded[0])
                require(match is not None, "iFlow skill base missing")
                base = Path(match[1].strip())
                require(base.is_absolute() and base.name == skill["name"] and
                        base.resolve() == skill["path"].resolve(), "wrong iFlow skill path")
                if kind == "native":
                    require(base == skill["path"], "iFlow bypassed its native skill directory")
                else:
                    require(base.parent.parent.name.startswith("shiju-session-skills-"),
                            "iFlow bypassed the session skill view")
                return self.call(tools, read, "read_file",
                                 {"absolute_path": str(base / "references/value.txt")})
            require(skill["attachment"] in self.result(messages, read)[0],
                    f"iFlow {kind} attachment was not read")

        if self.mounted:
            if "mcp" not in self.calls:
                return self.call(tools, "mcp", MCP_TOOL, {})
            result, _ = self.result(messages, "mcp")
            if self.rejected:
                require(re.search(r"reject|denied|cancel", result, re.I)
                        and self.fixtures["mcp"] not in result, "iFlow MCP refusal missing")
            else:
                require(self.fixtures["mcp"] in result, "iFlow MCP value was not read")
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def call(self, tools, kind, name, arguments):
        require(name in tools, f"iFlow {kind} tool missing")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= set(arguments) <= set(schema.get("properties", {})),
                f"iFlow {kind} tool schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.calls[kind] = call_id
        self.pending = kind
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def result(self, messages, kind):
        call_id = self.calls[kind]
        results = [(row["content"], index) for index, row in enumerate(messages)
                   if row["role"] == "tool" and row.get("tool_call_id") == call_id]
        require(len(results) == 1, f"iFlow {kind} result missing or ambiguous")
        self.completed[kind] = call_id
        content, index = results[0]
        return message_text(content), index

    def approves(self, metadata):
        return (self.pending == "mcp" and metadata.get("title") == MCP_TOOL
                and metadata.get("arguments") in (None, {}))

    def approves_skill(self, metadata):
        if self.pending in ("native", "selected"):
            skill = self.fixtures["skills"][self.pending]
            if (metadata.get("title") == "Launch skill: " + skill["name"]
                    and metadata.get("arguments") in (None, {"skill": skill["name"]})):
                return self.pending
        return None


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated iFlow model request")
            require(self.path == "/v1/chat/completions", "unexpected iFlow model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid iFlow request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid iFlow Chat Completions envelope")
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
