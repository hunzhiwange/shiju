"""CodeBuddy 2.179.3 native Skill and MCP over Chat Completions."""
import hashlib
import json
import os
from pathlib import Path
import platform
from queue import Empty
import re
import secrets
import shutil
import subprocess
import time

from verify_acp_integration import Process, ProviderHandler, require
from verify_acp_model_source import ROOT

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".codebuddy/skills"
MCP_TOOL = "mcp__coexistence__read_probe"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed CodeBuddy platform")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json")
                          .read_text())["codebuddy"]
    require(contract["version"] == "2.179.3", "CodeBuddy version differs from coexistence pin")
    require(shutil.which("node", path=env["PATH"]), "Node is unavailable")
    revision = "codebuddy-2.179.3-model-source-v2"
    if source := os.environ.get("CODEBUDDY_COEXISTENCE_RUNTIME"):
        runtime = Path(source).resolve()
    else:
        base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
        runtime = base / "acp-runtime" / revision
    require((runtime / "node_modules" / contract["package"]).is_dir(),
            "reviewed CodeBuddy package is unavailable")
    private = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision
    shutil.copytree(runtime, private, symlinks=True)
    package = private / "node_modules" / contract["package"]
    for name, digest in contract["files"].items():
        require(hashlib.sha256((package / name).read_bytes()).hexdigest() == digest,
                "CodeBuddy artifact digest differs from contract")
    entry = package / contract["bin"]
    version = subprocess.run([str(entry), "--version"], cwd=private, env=env,
                             capture_output=True, timeout=15, check=False)
    require(version.returncode == 0 and b"2.179.3" in version.stdout,
            "CodeBuddy entry did not report the pinned version")
    return {"command": str(entry), "args": ["--acp"], "env": {}}, {
        "version": contract["version"], "sha256": contract["files"]}


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.calls = {}
        self.completed = {}
        self.failure = None
        self.search_attempts = 0
        self.pending = None

    def response(self, body):
        require(body["model"] == MODEL, "unexpected upstream model")
        tools = {tool["function"]["name"]: tool["function"] for tool in body.get("tools", [])}
        if not tools:
            return {"content": "Ready"}, "stop"
        messages = body["messages"]
        self.requests += 1
        if self.requests == 1:
            request = json.dumps([messages, tools])
            description = tools.get("Skill", {}).get("description", "")
            catalog = set(re.findall(r"^- (\S+?):\s", "\n".join(
                re.findall(r"<available_skills>(.*?)</available_skills>", description, re.S)), re.M))
            for kind, skill in self.fixtures["skills"].items():
                discovered = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog) == discovered, f"unexpected {kind} skill catalog")
                require(skill["body"] not in request and skill["attachment"] not in request,
                        "skill content appeared before activation")
            require(self.fixtures["mcp"] not in request, "MCP value appeared before invocation")
        for kind in ("native", "selected") if self.mounted else ("native",):
            skill = self.fixtures["skills"][kind]
            if kind not in self.calls:
                return self.call(tools, kind, "Skill", {"skill": skill["name"]})
            expansion = self.result(messages, kind, skill["body"])
            read = kind + "_read"
            if read not in self.calls:
                match = re.search(r"Base directory for this skill: ([^\n]+)", expansion)
                require(match is not None, "native activation omitted the skill directory")
                base = Path(match[1].strip())
                require(base.is_absolute() and base.name == skill["name"] and
                        (base / "SKILL.md").resolve() == (skill["path"] / "SKILL.md").resolve(),
                        "unexpected skill read target")
                if kind == "native":
                    require(base == skill["path"], "native skill bypassed its project directory")
                else:
                    require(base.parent.name == "skills" and
                            base.parent.parent.name.startswith("shiju-session-skills-"),
                            "selected skill bypassed the session view")
                return self.call(tools, read, "Read", {"file_path": str(base / "references/value.txt")})
            self.result(messages, read, skill["attachment"])
        if "mcp" not in self.calls and "ToolSearch" in tools and MCP_TOOL not in tools:
            if "search" not in self.calls:
                return self.search(tools)
            result = self.result(messages, "search", record=False)
            if not re.search(r"^## " + re.escape(MCP_TOOL) + r"$", result, re.M):
                # Exact lookup can return a nearest-match tool when the name is absent.
                require(result.startswith(("No matching tools found", "Found ", "Not found")),
                        "unexpected ToolSearch result envelope")
                if self.mounted and self.search_attempts < 5:
                    time.sleep(0.25)
                    return self.search(tools)
                require(not self.mounted, "mounted MCP tool missing from delayed discovery")
            else:
                require(self.mounted, "unselected MCP tool was discoverable")
                schema = re.search(r"Parameters:\s*```json\s*(.*?)\s*```", result, re.S)
                require(schema is not None and not json.loads(schema[1]).get("required"),
                        "MCP schema missing or requires arguments")
                return self.call(tools, "mcp", "DeferExecuteTool",
                                 {"toolName": MCP_TOOL, "params": {}})
        elif "mcp" not in self.calls:
            require((MCP_TOOL in tools) == self.mounted, "unexpected MCP tool catalog")
        if self.mounted:
            if "mcp" not in self.calls:
                return self.call(tools, "mcp", MCP_TOOL, {})
            if self.rejected:
                result = self.result(messages, "mcp").lower()
                require(any(word in result for word in ("reject", "denied", "cancel")),
                        "MCP result lacks a rejection")
                require(self.fixtures["mcp"] not in result, "rejected MCP returned the fixture value")
            else:
                self.result(messages, "mcp", self.fixtures["mcp"])
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def search(self, tools):
        self.search_attempts += 1
        return self.call(tools, "search", "ToolSearch", {"tool_names": [MCP_TOOL]})

    def result(self, messages, kind, expected=None, *, record=True):
        call_id = self.calls[kind]
        matches = [message["content"] for message in messages if message.get("role") == "tool"
                   and message.get("tool_call_id") == call_id]
        require(len(matches) == 1, f"missing or ambiguous {kind} result")
        content = matches[0]
        require(isinstance(content, str), f"unexpected {kind} result envelope")
        require(expected is None or expected in content, f"{kind} result lacks expected content")
        if record:
            self.completed[kind] = call_id
        return content

    def call(self, tools, kind, name, arguments):
        require(name in tools, f"{kind} tool missing from native catalog")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", ())) <= set(arguments) <= set(schema.get("properties", {})),
                f"{kind} tool schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.calls[kind] = call_id
        self.pending = (kind, name, arguments)
        delta = {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                 "function": {"name": name, "arguments": json.dumps(arguments)}}]}
        if kind == "mcp":
            delta["content"] = "Checking the selected read-only MCP tool."
        return delta, "tool_calls"

    def approves(self, metadata):
        if self.pending is None or self.pending[0] != "mcp" or "mcp" in self.completed:
            return False
        _, name, arguments = self.pending
        return metadata.get("title") in (name, "ACP tool") and metadata.get("arguments") == arguments

    def approves_skill(self, metadata):
        if self.pending is None:
            return None
        kind, name, arguments = self.pending
        if (kind in ("native_read", "selected_read") and name == "Read" and kind not in self.completed
                and metadata.get("title") in ("Read", "ACP tool")
                and metadata.get("arguments") == arguments):
            return kind
        return None

    def expected_skill_approvals(self):
        # CodeBuddy asks for the projected attachment outside the project.
        return ["selected_read"] if self.mounted else []

    def observe_terminal(self, events):
        if not self.rejected or "mcp" in self.completed or self.pending is None:
            return
        # Native CodeBuddy cancels the prompt on refusal without a model
        # continuation. Match the pending tool ID and the cancellation.
        kind, name, arguments = self.pending
        call_id = self.calls.get("mcp")
        matching = [event for event in events if event.get("type") == "chat.tool"
                    and event.get("call_id") == call_id]
        attempted = [event["payload"].get("input") for event in matching
                     if event.get("status") == "running" and event["payload"].get("title") == name]
        cancelled = any(event.get("type") == "chat.step_finish" and event.get("finish_reason") == "cancelled"
                        for event in events)
        if (kind == "mcp" and call_id and cancelled
                and any(json.loads(value) == arguments for value in attempted if isinstance(value, str))
                and not any(event.get("status") == "completed" for event in matching)):
            self.completed["mcp"] = call_id


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated model request")
            require(self.path == "/v1/chat/completions", "unexpected CodeBuddy model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Chat Completions envelope")
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


def verify_native_plugins(spec, root, env, model, make_skill):
    # The session_model driver disables settings sources. Exercise CodeBuddy's
    # own user setting with an isolated native configuration as a separate control.
    config = root / "plugin-control-config"
    workspace = root / "plugin-control-workspace"
    config.mkdir()
    workspace.mkdir()
    (config / "settings.json").write_text(json.dumps({
        "enabledPlugins": {"disabled-probe@inline": False},
    }))
    argv = [spec["command"], "--acp", "--setting-sources", "user"]
    skills = {}
    for state in ("enabled", "disabled"):
        name = state + "-probe"
        plugin = root / name
        manifest = plugin / ".codebuddy-plugin"
        manifest.mkdir(parents=True)
        (manifest / "plugin.json").write_text(json.dumps({
            "name": name, "version": "1.0.0", "skills": "./skills/",
        }))
        skills[state] = make_skill(plugin / "skills", state)["name"]
        argv += ["--plugin-dir", str(plugin)]
    selected = make_skill(root / "plugin-control-view", "selected")
    plugin_env = dict(env, CODEBUDDY_CONFIG_DIR=str(config), WORKBUDDY_CONFIG_DIR=str(config),
                      CODEBUDDY_API_KEY=model.key,
                      CODEBUDDY_BASE_URL=f"http://127.0.0.1:{model.server_port}/v1",
                      CODEBUDDY_DISABLE_BUILTIN_MODELS="1", CODEBUDDY_DISABLE_CUSTOM_MODELS_FILE="1",
                      CODEBUDDY_MODEL=MODEL, CODEBUDDY_SMALL_FAST_MODEL=MODEL,
                      CODEBUDDY_FORCE_HEADLESS_BUNDLE="1", CODEBUDDY_FORCE_LITE_WB_BUNDLE="0")

    class PluginCatalog:
        failure = None
        checked = False

        def __init__(self, mounted):
            self.mounted = mounted

        def response(self, body):
            require(body["model"] == MODEL, "unexpected plugin control model")
            for tool in body.get("tools", []):
                target = tool["function"]
                if target["name"] == "Skill":
                    catalog = target["description"]
                    require(skills["enabled"] in catalog, "enabled inline plugin missing")
                    require(skills["disabled"] not in catalog, "disabled inline plugin was discoverable")
                    require((selected["name"] in catalog) == self.mounted,
                            "unexpected plugin control selection")
                    self.checked = True
            return {"content": "COEXISTENCE_DONE"}, "stop"

    evidence = []
    for mounted in (False, True):
        model.probe = probe = PluginCatalog(mounted)
        plugin_env["SHIJU_CODEBUDDY_SKILL_ROOT"] = str(selected["path"].parent) if mounted else ""
        cli = Process(argv, workspace, plugin_env, root / "codebuddy-plugin-control.log")
        try:
            cli.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
            session, _ = cli.call("session/new", {"cwd": str(workspace), "mcpServers": []})
            out, _ = cli.call("session/prompt", {"sessionId": session["sessionId"],
                              "prompt": [{"type": "text", "text": "Inspect the available skills."}]})
        except (AssertionError, Empty) as error:
            raise AssertionError(probe.failure or "native plugin ACP exchange failed") from error
        else:
            require(not probe.failure, probe.failure)
            require(probe.checked and out["stopReason"] == "end_turn", "plugin control incomplete")
            evidence.append({"mounted": mounted, "enabled_visible": True, "disabled_visible": False})
        finally:
            cli.close()
    return evidence
