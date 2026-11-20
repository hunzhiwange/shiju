"""Probe the reviewed Stakpak ACP skill discovery boundary."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess

from verify_acp_integration import require
from verify_acp_model_source import ROOT

from . import correlated_result
from .cortex_code import mcp_server  # Stakpak uses the same reviewed stdio sandbox.
from .opencode import Handler, Probe as ChatProbe

VERSION = "0.3.88"
REVISION = f"stakpak-{VERSION}-model-source-v3"
MODEL = "deepseek-v4.1-flash-1017"
MODEL_OUTPUT_LIMIT = 16_000
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
MCP_TOOL = "coexistence__read_probe"
NATIVE_DIR = ".stakpak/skills"
VIEW_TOOL = "stakpak__view"
LOAD_SKILL_TOOL = "stakpak__load_skill"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Stakpak platform")
    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(manifest.read_text())["stakpak"]
    require(contract["version"] == VERSION, "unexpected Stakpak contract version")
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = Path(os.environ.get("ACP_STAKPAK_REVIEWED_DIR", base / "acp-runtime" / REVISION))
    require(source.is_dir(), "reviewed Stakpak runtime unavailable")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    runtime.mkdir(parents=True, mode=0o700)
    for name, expected in contract["files"].items():
        entry = runtime / name
        shutil.copy2(source / name, entry)
        digest = hashlib.sha256()
        with entry.open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(block)
        require(digest.hexdigest() == expected, f"Stakpak artifact digest mismatch: {name}")
    command = runtime / contract["bin"]
    try:
        version = subprocess.run([str(command), "--version"], env=env,
                                 capture_output=True, timeout=15)
    except subprocess.TimeoutExpired:
        raise AssertionError("Stakpak version check timed out") from None
    require(version.returncode == 0 and version.stdout.strip() == f"stakpak {VERSION}".encode(),
            "unexpected Stakpak executable version")
    return {"command": str(command), "args": ["acp"]}, {
        "version": VERSION, "revision": REVISION, "platform": "darwin-arm64",
        "sha256": contract["files"][contract["bin"]], "files": contract["files"],
    }


class Probe(ChatProbe):
    def response(self, body):
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if not tools:
            return {"content": "Stakpak fixture session"}, "stop"
        require(body["model"] == MODEL, "unexpected Stakpak model")
        self.requests += 1
        native = self.fixtures["skills"]["native"]
        if self.requests == 1:
            envelope = json.dumps(body)
            for skill in self.fixtures["skills"].values():
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Stakpak exposed skill content before activation")
                require(skill["name"] not in envelope, "Stakpak now advertises skill metadata")
            require(self.fixtures["mcp"] not in envelope, "Stakpak exposed MCP data before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected Stakpak MCP catalog")
            require(not any(name.rsplit("__", 1)[-1] == "load_skill" for name in tools),
                    "Stakpak now advertises load_skill and needs a new review")
            # Use the real read path only to establish a baseline. The known
            # fixture path does not demonstrate native discovery or activation.
            return self.call(tools, "ordinary_read", VIEW_TOOL,
                             {"path": str(native["path"] / "references/value.txt")})

        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"])
                   for row in body["messages"] if row["role"] == "tool"]
        correlated_result(results, call_id, native["attachment"])
        self.completed[step] = call_id
        self.pending = None
        self.failure = "Stakpak ACP model request lacks a native skill catalog and activation tool"
        return {"content": "STAKPAK_READ_BASELINE_DONE"}, "stop"

    def observe_terminal(self, events):
        if self.mounted and any(
            row.get("type") == "chat.error"
            and "stakpak: no verified skill directory mount branch" in str(row.get("error", ""))
            for row in events
        ):
            self.failure = "Stakpak host rejected the unverified skill mount before model inference"
        return False

    def approves(self, _metadata):
        return False


def native_control():
    """Isolate TOML tool selection from gateway and skill-directory discovery."""
    import secrets
    import tempfile

    from verify_acp_approval import environment
    from verify_acp_integration import Process
    from verify_acp_skill_coexistence import make_skill, model_server

    class LoaderControl(ChatProbe):
        def response(self, body):
            tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
            if not tools:
                return {"content": "Stakpak fixture session"}, "stop"
            require(body["model"] == MODEL, "unexpected Stakpak model")
            self.requests += 1
            if self.pending is not None:
                kind, _, _, call_id = self.pending
                results = [dict(row, tool_use_id=row["tool_call_id"])
                           for row in body["messages"] if row["role"] == "tool"]
                skill = self.fixtures[kind]
                result = str(correlated_result(results, call_id)["content"])
                if kind == "outside":
                    require("LOAD_SKILL_ERROR" in result and skill["body"] not in result,
                            "Stakpak exposed a skill outside its native roots")
                else:
                    require(skill["body"] in result and skill["attachment"] not in result,
                            "Stakpak did not lazily load the requested skill")
                self.completed[kind] = call_id
            else:
                envelope = json.dumps(body)
                for skill in self.fixtures.values():
                    require(all(skill[key] not in envelope for key in ("name", "body", "attachment")),
                            "Stakpak ACP published a fixture in its initial model request")
                require({VIEW_TOOL, LOAD_SKILL_TOOL} <= tools.keys(), "missing native TOML control tool")
            for kind, skill in self.fixtures.items():
                if kind not in self.completed:
                    return self.call(tools, kind, LOAD_SKILL_TOOL, {"uri": str(skill["path"] / "SKILL.md")})
            self.pending = None
            return {"content": "STAKPAK_TOML_CONTROL_DONE"}, "stop"

    with tempfile.TemporaryDirectory(prefix="shiju-stakpak-native-") as directory:
        root = Path(directory).resolve()
        env = environment(root)
        spec, artifact = launch(env)
        workspace = root / "workspace"
        workspace.mkdir(mode=0o700)
        home = Path(env["HOME"])
        fixtures = {"project": make_skill(workspace / NATIVE_DIR, "project"),
                    "home": make_skill(home / NATIVE_DIR, "home"),
                    "outside": make_skill(root / "outside", "outside")}
        key = secrets.token_hex(24)
        with model_server(Handler, key) as server:
            server.probe = probe = LoaderControl(fixtures, False, False)
            config = home / ".stakpak"
            cache = config / "cache"
            cache.mkdir(mode=0o700)
            path = config / "config.toml"
            path.touch(mode=0o600)
            path.write_text(f"""
[profiles.default]
provider = "local"
allowed_tools = {json.dumps([VIEW_TOOL, LOAD_SKILL_TOOL])}
[profiles.default.providers.shiju-session]
type = "custom"
api_endpoint = "http://127.0.0.1:{server.server_port}/v1"
api_key = "{key}"
[settings]
collect_telemetry = false
auto_append_gitignore = false
""")
            (cache / "models.json").write_text(json.dumps({"fetched_at": 0, "providers": {"openai": {
                "id": "openai", "name": "OpenAI compatible", "models": {MODEL: {
                    "id": MODEL, "name": MODEL, "provider": "shiju-session", "reasoning": False,
                    "limit": {"context": 100000, "output": MODEL_OUTPUT_LIMIT},
                }},
            }}}))
            env.update(SHIJU_STAKPAK_RUNTIME=str(home), VW_ACP_LAUNCH_REFERENCE="fixture")
            process = Process([spec["command"], *spec["args"]], workspace, env, root / "stderr")
            try:
                process.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
                session, _ = process.call("session/new", {"cwd": str(workspace), "mcpServers": []})
                require(session["models"]["currentModelId"] == MODEL, "native control model binding changed")
                result, events = process.call("session/prompt", {"sessionId": session["sessionId"],
                    "prompt": [{"type": "text", "text": "Check available read-only fixture resources."}]})
                require(result["stopReason"] == "end_turn" and probe.failure is None
                        and set(probe.completed) == fixtures.keys(), "Stakpak TOML control did not complete")
                require(not any("id" in row and "method" in row for row in events),
                        "unexpected native control host callback")
            finally:
                process.close()
    return {"artifact": artifact, "stop_reason": "end_turn", "model": MODEL,
            "initial_skill_metadata": False, "load_skill": LOAD_SKILL_TOOL, "outside_root_rejected": True,
            "calls": probe.completed, "model_requests": probe.requests}
