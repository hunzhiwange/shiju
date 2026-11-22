"""Probe Factory Droid's reviewed native Skill, Read, and stdio MCP paths."""

import hashlib
from contextlib import contextmanager
from inspect import getsource
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess
import sys
import tempfile

from acp_mount_integration import rpc
from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

from . import correlated_result
from .mimo_code import mcp_server as darwin_mcp_server

MODEL = "deepseek-v4.1-flash-1017"
MODEL_REASONING = True
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".factory/skills"
VERSION = "1.9.0"
REVISION = f"droid-{VERSION}-model-source-v2"
MCP_TOOL = "coexistence___read_probe"
MCP_TITLE = "coexistence: Read probe"
MANAGED_DIR = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"


def launch(env):
    if platform.system() == "Linux":
        return launch_linux(env)
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Droid platform")
    manifest = MANAGED_DIR / "contracts.json"
    contract = json.loads(manifest.read_text())["droid"]
    require(contract["version"] == VERSION, "unexpected Droid artifact version")
    target = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    target.parent.mkdir()
    entry = target / contract["bin"]
    cache = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    cached_entry = cache / "acp-runtime" / REVISION / entry.name
    if cached_entry.is_file():
        target.mkdir(mode=0o700)
        shutil.copy2(cached_entry, entry)
    else:
        with tempfile.TemporaryDirectory(dir=env["TMPDIR"]) as directory:
            assets = Path(directory)
            shutil.copy2(manifest, assets)
            shutil.copytree(ROOT / "patches/acp-model-source/droid", assets / "droid")
            try:
                install = subprocess.run(
                    [sys.executable, str(MANAGED_DIR / "install.py"), str(assets), str(target), "droid"],
                    env=env, capture_output=True, timeout=180)
            except subprocess.TimeoutExpired as error:
                raise AssertionError("Droid reviewed runtime installation timed out") from error
            require(install.returncode == 0, "Droid reviewed runtime installation failed")
    digest = hashlib.sha256()
    with entry.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    require(digest.hexdigest() == contract["files"][entry.name], "Droid executable digest mismatch")
    signature = subprocess.run(["/usr/bin/codesign", "--verify", str(entry)],
                               env=env, capture_output=True)
    require(signature.returncode == 0, "Droid executable signature check failed")
    version = subprocess.run([str(entry), "--version"], env=env, capture_output=True, timeout=15)
    require(version.returncode == 0 and version.stdout.strip() == VERSION.encode(),
            "unexpected Droid executable version")
    return {"command": str(entry), "args": ["exec", "--output-format", "acp"]}, {
        "version": VERSION, "revision": REVISION, "sha256": digest.hexdigest(),
    }


def launch_linux(env):
    manifest = MANAGED_DIR / "contracts.json"
    patches = ROOT / "patches/acp-model-source/droid"
    contract = json.loads((patches / "linux.json").read_text())
    target = contract["targets"].get(platform.machine())
    require(target is not None, "unreviewed Linux Droid architecture")
    require(contract["version"] == VERSION, "unexpected Droid artifact version")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / contract["revision"]
    runtime.parent.mkdir()
    with tempfile.TemporaryDirectory(dir=env["TMPDIR"]) as directory:
        assets = Path(directory)
        shutil.copy2(manifest, assets)
        shutil.copytree(patches, assets / "droid")
        installed = subprocess.run(
            [sys.executable, str(MANAGED_DIR / "install.py"), str(assets), str(runtime), "droid"],
            env=env, capture_output=True, timeout=300)
    require(installed.returncode == 0, "Droid reviewed Linux runtime installation failed")
    entry = runtime / contract["bin"]
    digest = hashlib.sha256()
    with entry.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    require(digest.hexdigest() == target["patched_sha256"], "Droid Linux executable digest mismatch")
    version = subprocess.run([str(entry), "--version"], env=env, capture_output=True, timeout=15)
    require(version.returncode == 0 and version.stdout.strip() == VERSION.encode(),
            "unexpected Droid Linux executable version")
    return {"command": str(entry), "args": ["exec", "--output-format", "acp"]}, {
        "version": VERSION, "revision": contract["revision"], "entry": entry.name,
        "sha256": digest.hexdigest(), "model_endpoint": API_PATH + "/chat/completions",
    }


@contextmanager
def mcp_server(workspace, attachment, audit):
    if platform.system() != "Linux":
        with darwin_mcp_server(workspace, attachment, audit) as server:
            yield server
        return
    script = workspace / "mcp-fixture.py"
    script.write_text(
        "import json, os, sys\nfrom pathlib import Path\n"
        "def require(condition, message):\n"
        "    if not condition:\n"
        "        raise AssertionError(message)\n\n"
        + getsource(rpc)
        + "\naudit = Path(os.environ['TMPDIR']) / 'mimo-fixture-audit'\n"
        "audit.touch(exist_ok=False)\n"
        "for line in sys.stdin:\n"
        "    reply = rpc(json.loads(line), sys.argv[1], audit, 'stdio')\n"
        "    if reply is not None:\n"
        "        print(json.dumps(reply), flush=True)\n")
    payload = workspace / "mcp-payload.txt"
    payload.write_text(attachment.read_text())
    yield {"name": "coexistence", "command": str(Path(sys.executable).resolve(strict=True)),
           "args": ["-I", "-B", str(script), str(payload)], "env": []}


def prepare_workspace(workspace):
    disabled = workspace / NATIVE_DIR / "droid-probe-disabled"
    disabled.mkdir()
    (disabled / "SKILL.md").write_text(
        "---\nname: droid-probe-disabled\ndescription: Disabled project fixture\n---\nThis valid fixture is disabled.\n")
    (workspace / ".factory/settings.json").write_text(
        json.dumps({"disabledSkills": [disabled.name]}))


class Probe:
    def __init__(self, fixtures, mounted, rejected, *, observe_stdio=True):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.failure = None
        self.pending = None
        self.observe_stdio = observe_stdio
        self.audit = None

    def response(self, body):
        require(body["model"] == MODEL and body.get("stream") is True,
                "unexpected Droid Chat Completions contract")
        tools = {row["function"]["name"]: row["function"] for row in body.get("tools", [])}
        if not tools:
            return {"content": "Fixture session"}, "stop"
        self.requests += 1
        messages = body["messages"]
        require((MCP_TOOL in tools) == self.mounted, "unexpected Droid stdio MCP catalog")
        skills = self.fixtures["skills"]
        if self.requests == 1:
            require({"Skill", "Read", "ToolSearch"} <= tools.keys(),
                    "Droid native tools missing")
            text = []
            for row in messages:
                content = row.get("content")
                if isinstance(content, str):
                    text.append(content)
                elif isinstance(content, list):
                    text.extend(part.get("text", "") for part in content if part.get("type") == "text")
            catalog = "\n".join(match for message in text for match in re.findall(
                r"<system-reminder>\nAvailable skills \([^\n]+\):\n(.*?)</system-reminder>",
                message, re.S))
            names = set(re.findall(r"^([a-z0-9-]+): ", catalog, re.M))
            require("droid-probe-disabled" not in names, "Droid ignored project skill disablement")
            envelope = json.dumps(body)
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in names) == visible, f"unexpected Droid {kind} skill catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Droid skill content appeared before activation")
            require(self.fixtures["mcp"] not in envelope, "Droid MCP content appeared before invocation")
            if self.mounted and self.observe_stdio:
                root = skills["native"]["path"].parents[3]
                audits = list((root / "tmp").glob("shiju-mimo-mcp-*/mimo-fixture-audit"))
                require(len(audits) == 1, "missing Droid stdio audit")
                self.audit = root / ("droid-rejected-audit" if self.rejected else "droid-mounted-audit")
                os.link(audits[0], self.audit)
                require(not self.audit.read_text(), "Droid executed MCP before approval")
            return self.call(tools, "native", "Skill", {"skill": skills["native"]["name"]})

        step, name, arguments = self.pending
        # Droid assigns its own IDs when serializing assistant tool calls back to the model.
        calls = [call for row in messages if row["role"] == "assistant"
                 for call in row.get("tool_calls", [])]
        require(calls and calls[-1]["function"]["name"] == name
                and json.loads(calls[-1]["function"]["arguments"]) == arguments,
                "Droid continuation omitted the assistant tool call")
        call_id = calls[-1]["id"]
        results = [dict(row, tool_use_id=row["tool_call_id"]) for row in messages if row["role"] == "tool"]
        if step == "mcp":
            value = self.fixtures["mcp"]
            result = str(correlated_result(results, call_id)["content"])
            require((value not in result and re.search(r"reject|denied|cancel", result, re.I))
                    if self.rejected else value in result, "unexpected Droid MCP continuation")
        else:
            kind = step.removesuffix("_read")
            skill = skills[kind]
            field = "attachment" if step.endswith("_read") else "body"
            result = str(correlated_result(results, call_id, skill[field])["content"])
        self.completed[step] = call_id

        if step in ("native", "selected"):
            match = re.search(r'<skill name="' + re.escape(skill["name"]) + r'" filePath="([^"]+)">', result)
            require(match is not None, "Droid activation omitted its skill path")
            path = Path(match[1])
            require(path.is_absolute() and path.name == "SKILL.md", "invalid Droid skill path")
            if step == "native":
                require(path.resolve() == (skill["path"] / "SKILL.md").resolve(),
                        "unexpected Droid native skill path")
            else:
                home = path.parent.parent.parent.parent
                require(home.name.startswith("shiju-acp-launch-")
                        and path == home / NATIVE_DIR / skill["name"] / "SKILL.md",
                        "Droid selected skill did not use the isolated Factory root")
                view = (home / NATIVE_DIR).resolve()
                require(view.parent.name.startswith("shiju-session-skills-") and view.name == "skills"
                        and path.resolve() == (skill["path"] / "SKILL.md").resolve(),
                        "Droid selected skill bypassed its session view")
            return self.call(tools, step + "_read", "Read",
                             {"file_path": str(path.parent / "references/value.txt")})
        if self.mounted and step == "native_read":
            return self.call(tools, "selected", "Skill", {"skill": skills["selected"]["name"]})
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def call(self, tools, step, name, arguments):
        require(name in tools, f"Droid {step} tool missing")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= arguments.keys() <= schema.get("properties", {}).keys(),
                f"unexpected Droid {step} tool schema")
        self.pending = (step, name, arguments)
        delta = {"tool_calls": [{"index": 0, "id": "call_" + secrets.token_hex(12), "type": "function",
                                 "function": {"name": name, "arguments": json.dumps(arguments)}}]}
        if step == "mcp":
            # The same model preamble precedes both outcomes. A native refusal
            # ends the prompt without another model response.
            delta["content"] = "Checking the selected read-only MCP tool."
        return delta, "tool_calls"

    def approves(self, metadata):
        return (self.pending == ("mcp", MCP_TOOL, {})
                and metadata.get("title") == "coexistence: read_probe" and metadata.get("arguments") == {})

    def observe_terminal(self, events):
        if not self.rejected or "mcp" in self.completed or self.pending != ("mcp", MCP_TOOL, {}):
            return False
        # Droid can cancel the prompt before sending a model continuation.
        calls = [event for event in events if event.get("type") == "chat.tool"
                 and event.get("payload", {}).get("title") == MCP_TITLE]
        failures = [event for event in calls if event.get("kind") == "complete"
                    and event.get("status") == "failed"
                    and event["payload"].get("error") == "Error: Tool execution cancelled by user"]
        if len(failures) != 1:
            return False
        call_id = failures[0].get("call_id")
        if not call_id or not any(event.get("call_id") == call_id and event.get("status") == "running"
                                  for event in calls):
            return False
        self.completed["mcp"] = call_id
        return True

    def mcp_trace(self, events):
        audit = self.audit.read_text().splitlines() if self.mounted else []
        if platform.system() == "Linux":
            root = self.fixtures["skills"]["native"]["path"].parents[3]
            approved = root / "droid-mounted-audit"
            if approved.exists() and approved != self.audit:
                require(approved.read_text().splitlines() == ["stdio"],
                        "Droid executed a previously mounted MCP service again")
            if self.mounted:
                call_id = self.completed["mcp"]
                status = "failed" if self.rejected else "completed"
                require(any(event.get("type") == "chat.tool" and event.get("call_id") == call_id
                            and event.get("status") == status for event in events),
                        "Droid terminal MCP event did not match its tool call")
        return audit, self.completed.get("mcp")


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.path == "/v1/chat/completions", "unexpected Droid model protocol")
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Droid model request")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Droid model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Droid model envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.chunk(body["model"], dict(delta, role="assistant"), None)
        self.chunk(body["model"], {}, finish)
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()
