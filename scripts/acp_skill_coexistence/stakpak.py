"""Probe the reviewed Stakpak native ACP skills and read-only MCP."""
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess

from verify_acp_integration import require
from verify_acp_model_source import ROOT

from . import correlated_result
from .cortex_code import mcp_server  # Stakpak uses the same reviewed stdio sandbox.
from .opencode import Handler, Probe as ChatProbe

VERSION = "0.3.88"
REVISION = f"stakpak-{VERSION}-model-source-v4"
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
    system, machine = platform.system(), platform.machine()
    linux = system == "Linux" and machine in {"x86_64", "aarch64"}
    require(linux or (system, machine) == ("Darwin", "arm64"),
            "unreviewed Stakpak platform")
    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(manifest.read_text())["stakpak"]
    require(contract["version"] == VERSION, "unexpected Stakpak contract version")
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    configured = os.environ.get("ACP_STAKPAK_REVIEWED_DIR")
    if linux:
        require(os.geteuid() != 0 and configured and Path(configured).is_absolute(),
                "run as non-root and set ACP_STAKPAK_REVIEWED_DIR to the private Linux source build")
    source = Path(configured) if configured is not None else base / "acp-runtime" / REVISION
    require(source.is_dir(), "reviewed Stakpak runtime unavailable")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    runtime.mkdir(parents=True, mode=0o700)
    receipt = json.loads((source / "build-receipt.json").read_text())
    require(all(receipt.get(key) == contract[key]
                for key in ("version", "source_sha256", "patch_sha256")),
            "Stakpak build receipt differs from reviewed contract")
    files = {**contract["files"], "stakpak-native": receipt["sha256"]}
    for name, expected in files.items():
        entry = runtime / name
        shutil.copy2(source / name, entry)
        digest = hashlib.sha256()
        with entry.open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(block)
        require(digest.hexdigest() == expected, f"Stakpak artifact digest mismatch: {name}")
    (runtime / "build-receipt.json").write_text(json.dumps(receipt))
    command = runtime / contract["bin"]
    if not linux:
        try:
            version = subprocess.run([str(command), "--version"], env=env,
                                     capture_output=True, timeout=15)
        except subprocess.TimeoutExpired:
            raise AssertionError("Stakpak version check timed out") from None
        require(version.returncode == 0 and version.stdout.strip() == f"stakpak {VERSION}".encode(),
                "unexpected Stakpak executable version")
    artifact = {
        "version": VERSION, "revision": REVISION,
        "platform": f"Linux-{machine}" if linux else "darwin-arm64",
        "sha256": contract["files"][contract["bin"]], "files": files,
        "source_sha256": receipt["source_sha256"], "patch_sha256": receipt["patch_sha256"],
    }
    if linux:
        artifact["model_protocol"] = "POST /v1/chat/completions"
    return {"command": str(command), "args": ["acp"]}, artifact


DISABLED = "stakpak-disabled-probe"


def prepare_workspace(workspace):
    disabled = workspace / NATIVE_DIR / DISABLED
    disabled.mkdir()
    (disabled / "SKILL.md").write_text(
        f"---\nname: {DISABLED}\ndescription: Disabled fixture\n"
        "disable-model-invocation: true\n---\nDisabled.\n")


def skill_catalog(messages):
    system = "\n".join(str(row["content"]) for row in messages if row["role"] == "system")
    block = re.search(r"<available_skills>\n(.*?)\n</available_skills>", system, re.S)
    require(block is not None, "Stakpak model request lacks the native skill catalog")
    return {row["name"]: Path(row["uri"]) for row in json.loads(block[1])}


class Probe(ChatProbe):
    def response(self, body):
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if not tools:
            return {"content": "Stakpak fixture session"}, "stop"
        require(body["model"] == MODEL, "unexpected Stakpak model")
        self.requests += 1
        messages = body["messages"]
        skills = self.fixtures["skills"]
        catalog = skill_catalog(messages)
        if self.requests == 1:
            envelope = json.dumps(body)
            expected = {"native", "selected"} if self.mounted else {"native"}
            require(set(catalog) == {skills[kind]["name"] for kind in expected},
                    "unexpected Stakpak skill catalog")
            for skill in skills.values():
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Stakpak exposed skill content before activation")
            require(self.fixtures["mcp"] not in envelope, "Stakpak exposed MCP data before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected Stakpak MCP catalog")
            native = catalog[skills["native"]["name"]]
            require(native == skills["native"]["path"] / "SKILL.md", "unexpected native skill path")
            if self.mounted:
                selected = catalog[skills["selected"]["name"]]
                require(selected.is_absolute() and selected.parts[-4:] == (
                    ".stakpak", "skills", skills["selected"]["name"], "SKILL.md"
                ) and selected != skills["selected"]["path"] / "SKILL.md",
                    "Stakpak selected skill bypassed the private snapshot")
            return self.call(tools, "native", LOAD_SKILL_TOOL, {"uri": str(native)})

        step, _, arguments, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"])
                   for row in messages if row["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        self.completed[step] = call_id
        if step in ("native", "selected"):
            require(skills[step]["body"] in result and skills[step]["attachment"] not in result,
                    f"unexpected Stakpak {step} activation output")
            directory = Path(arguments["uri"]).parent
            require(f"Skill directory: {directory}" in result,
                    "Stakpak load_skill returned a different resource directory")
            return self.call(tools, step + "_read", VIEW_TOOL,
                             {"path": str(directory / "references/value.txt")})
        if step == "mcp":
            value = self.fixtures["mcp"]
            require(("TOOL_CALL_REJECTED" in result and value not in result)
                    if self.rejected else value in result, "unexpected Stakpak MCP result")
        else:
            require(skills[step.removesuffix("_read")]["attachment"] in result,
                    f"Stakpak {step} did not return its attachment")
        if self.mounted and step == "native_read":
            return self.call(tools, "selected", LOAD_SKILL_TOOL,
                             {"uri": str(catalog[skills["selected"]["name"]])})
        if step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def approves_skill(self, metadata):
        if self.pending is not None:
            step, _, arguments, _ = self.pending
            if (step in ("native", "selected") and metadata.get("title") == "Load skill"
                    and metadata.get("arguments") == arguments):
                return step
        return None

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == "Read Probe" and metadata.get("arguments") == {})


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
                catalog = skill_catalog(body["messages"])
                require(set(catalog) == {self.fixtures[kind]["name"] for kind in ("project", "home")},
                        "Stakpak native control discovered an unexpected skill")
                for skill in self.fixtures.values():
                    require(all(skill[key] not in envelope for key in ("body", "attachment")),
                            "Stakpak ACP exposed a fixture before activation")
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
        prepare_workspace(workspace)
        outside = fixtures["outside"]
        link = workspace / NATIVE_DIR / outside["name"]
        link.symlink_to(outside["path"], target_is_directory=True)
        outside["path"] = link
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
            command = [spec["command"], *spec["args"]]
            process = Process(command, workspace, env, root / "stderr")
            try:
                process.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
                session, _ = process.call("session/new", {"cwd": str(workspace), "mcpServers": []})
                require(session["models"]["currentModelId"] == MODEL, "native control model binding changed")
            finally:
                process.close()
            process = Process(command, workspace, env, root / "restore.stderr")
            try:
                process.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
                restored, _ = process.call("session/load", {
                    "sessionId": session["sessionId"], "cwd": str(workspace), "mcpServers": [],
                })
                require(restored["models"]["currentModelId"] == MODEL, "restored Stakpak model binding changed")
                approvals = []

                def approve(frame):
                    if "id" not in frame or "method" not in frame:
                        return
                    kind, _, arguments, _ = probe.pending
                    require(frame["method"] == "session/request_permission"
                            and kind in ("project", "home"), "unexpected native control host callback")
                    params = frame["params"]
                    tool = params["toolCall"]
                    require(params["sessionId"] == session["sessionId"]
                            and tool.get("title") == "Load skill" and tool.get("rawInput") == arguments,
                            "native load approval scope changed")
                    once = [item["optionId"] for item in params["options"] if item["kind"] == "allow_once"]
                    require(len(once) == 1, "native load is missing single-use approval")
                    reply = {"jsonrpc": "2.0", "id": frame["id"], "result": {
                        "outcome": {"outcome": "selected", "optionId": once[0]},
                    }}
                    process.child.stdin.write(json.dumps(reply) + "\n")
                    process.child.stdin.flush()
                    approvals.append(kind)

                result, _ = process.call("session/prompt", {"sessionId": session["sessionId"],
                    "prompt": [{"type": "text", "text": "Check available read-only fixture resources."}]},
                    on_frame=approve)
                require(result["stopReason"] == "end_turn" and probe.failure is None
                        and set(probe.completed) == fixtures.keys(), "Stakpak TOML control did not complete")
                require(approvals == ["project", "home"], "missing native load approvals")
            finally:
                process.close()
    return {"artifact": artifact, "stop_reason": "end_turn", "model": MODEL,
            "restored": True, "initial_skill_metadata": True, "load_skill": LOAD_SKILL_TOOL, "outside_root_rejected": True,
            "calls": probe.completed, "model_requests": probe.requests, "approvals": approvals}
