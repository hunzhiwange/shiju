"""Exercise the fixed Linux Snow installation and chat through the gateway."""

from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import secrets
import shutil
import sys
import tempfile
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from acp_skill_coexistence import snow
from verify_acp_approval import environment, gateway, request
from verify_acp_integration import Process, require
from verify_acp_model_source import ROOT
from verify_acp_skill_coexistence import mcp_server as session_mcp_server, model_server, run

REVISION = "snow-0.9.14-model-source-v5"
NATIVE_MCP = "nativeprobe-read_probe"
PROJECTED_MCP = "mcp__coexistence__read_probe"


@contextmanager
def mcp_server(workspace, attachment, audit):
    # Register a different service in Snow's own project configuration.
    native = workspace / ".snow"
    value = native / "probe-value.txt"
    value.write_text(secrets.token_hex(24))
    settings = native / "settings.json"
    config = json.loads(settings.read_text())
    config["mcpServers"] = {"nativeprobe": {
        "type": "stdio", "command": sys.executable,
        "args": [str(ROOT / "scripts/acp_mount_integration.py"), "--mcp-stdio",
                 str(value), str(native / "probe-audit.txt")],
    }}
    settings.write_text(json.dumps(config))
    (native / "permissions.json").write_text(json.dumps({
        "alwaysApprovedTools": [snow.SKILL_TOOL, snow.READ_TOOL, NATIVE_MCP],
    }))
    with session_mcp_server("stdio", attachment, audit) as spec:
        yield spec


class Probe(snow.Probe):
    def __init__(self, fixtures, mounted, rejected):
        super().__init__(fixtures, mounted, rejected)
        self.native = fixtures["skills"]["native"]["path"].parents[1]
        self.native_calls = self.audit()
        self.arguments = None

    def audit(self):
        path = self.native / "probe-audit.txt"
        return path.read_text().splitlines() if path.exists() else []

    def response(self, body):
        tools = {row["function"]["name"]: row["function"] for row in body["tools"]}
        require(NATIVE_MCP in tools, "Snow omitted its configured native MCP")
        if self.requests == 1:
            require(self.audit() == self.native_calls, "native MCP ran before approval")
            self.requests += 1
            return self.call(tools, "native_mcp", NATIVE_MCP, {})
        if self.pending and self.pending[0] == "native_mcp":
            results = [row["content"] for row in body["messages"]
                       if row.get("role") == "tool" and row.get("tool_call_id") == self.pending[2]]
            require(len(results) == 1 and (self.native / "probe-value.txt").read_text() in results[0],
                    "native MCP output did not reach the model")
            require(self.audit() == self.native_calls + ["stdio"], "unexpected native MCP audit")
            self.requests += 1
            return self.activate(tools, "native")
        return super().response(body)

    def call(self, tools, kind, name, arguments):
        self.arguments = arguments
        return super().call(tools, kind, name, arguments)

    def approves_skill(self, metadata):
        if self.pending and self.pending[0] != "mcp":
            kind, name, _ = self.pending
            if metadata.get("title") == name and metadata.get("arguments") == self.arguments:
                return kind
        return None

    def expected_skill_approvals(self):
        return ["native_mcp", *super().expected_skill_approvals()]


class CancelProbe:
    failure = None

    def __init__(self):
        self.catalogs = []

    def response(self, body):
        tools = {row["function"]["name"] for row in body["tools"]}
        self.catalogs.append(tools)
        if PROJECTED_MCP not in tools:
            return {"content": "MOUNT_REMOVED"}, "stop"
        return {"tool_calls": [{"index": 0, "id": "call_cancel", "type": "function",
                                "function": {"name": PROJECTED_MCP, "arguments": "{}"}}]}, "tool_calls"


class CancelHandler(snow.Handler):
    def do_POST(self):
        if self.path != "/shiju-vtcode-mcp/coexistence":
            return super().do_POST()
        if self.headers.get("Authorization") != "Bearer " + self.server.mcp_key:
            self.send_error(401)
            return
        payload = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.server.mcp_methods.append(payload["method"])
        if payload["method"] != "tools/list":
            self.send_error(403)
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({"jsonrpc": "2.0", "id": payload["id"], "result": {
            "tools": [{"name": "read_probe", "inputSchema": {"type": "object", "properties": {}}}],
        }}).encode())


def verify_cancel_and_load(entry, root, env):
    workspace = root / "cancel-workspace"
    workspace.mkdir()
    config = root / "cancel-snow"
    config.mkdir(mode=0o700)
    key = secrets.token_hex(24)
    probe = CancelProbe()
    with model_server(CancelHandler, key) as model:
        model.probe = probe
        model.mcp_methods = []
        model.mcp_key = secrets.token_hex(32)
        mcp = {"type": "http", "name": "coexistence",
               "url": f"http://127.0.0.1:{model.server_port}/shiju-vtcode-mcp/coexistence",
               "headers": [{"name": "Authorization", "value": "Bearer " + model.mcp_key}]}
        (config / "config.json").write_text(json.dumps({"snowcfg": {
            "baseUrl": f"http://127.0.0.1:{model.server_port}/v1", "baseUrlMode": "base",
            "apiKey": key, "advancedModel": snow.MODEL, "requestMethod": "chat",
            "enablePromptOptimization": False, "maxRetries": 0,
        }}))
        process = Process([str(entry), "--acp"], workspace,
                          dict(env, SNOW_CONFIG_DIR=str(config), SHIJU_SNOW_SKILL_ROOT=""),
                          root / "cancel-snow.log")
        try:
            info, _ = process.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
            require(info["agentInfo"]["version"] == "0.9.14", "unexpected ACP Snow version")
            raw_started = root / "unexpected-stdio-start"
            raw = {"name": "raw", "command": sys.executable, "env": [], "args": [
                "-c", "from pathlib import Path; import sys; Path(sys.argv[1]).touch()", str(raw_started),
            ]}
            raw_session, _ = process.call("session/new", {"cwd": str(workspace), "mcpServers": [raw]})
            prompt_content = [{"type": "text", "text": "Check the fixture."}]
            try:
                process.call("session/prompt", {"sessionId": raw_session["sessionId"],
                                                "prompt": prompt_content})
            except AssertionError as error:
                require("Invalid Snow host MCP selection" in str(error), "unexpected raw stdio error")
            else:
                raise AssertionError("Snow accepted raw session stdio")
            require(not raw_started.exists() and not probe.catalogs and not model.mcp_methods,
                    "raw session stdio crossed Snow's request boundary")

            session, _ = process.call("session/new", {"cwd": str(workspace), "mcpServers": [mcp]})
            sid = session["sessionId"]
            prompt = {"sessionId": sid, "prompt": prompt_content}
            approvals = []

            def cancel_then_approve(frame):
                if frame.get("method") != "session/request_permission":
                    return
                call = frame["params"]["toolCall"]
                require(call["toolCallId"] == "call_cancel" and call["title"] == PROJECTED_MCP
                        and call["rawInput"] == {}, "unexpected pending Snow MCP action")
                approvals.append(call["toolCallId"])
                process.send("session/cancel", {"sessionId": sid}, notification=True)
                reply = {"jsonrpc": "2.0", "id": frame["id"], "result": {
                    "outcome": {"outcome": "selected", "optionId": "approve_once"},
                }}
                process.child.stdin.write(json.dumps(reply) + "\n")
                process.child.stdin.flush()

            result, _ = process.call("session/prompt", prompt, cancel_then_approve)
            require(result["stopReason"] == "cancelled" and approvals == ["call_cancel"]
                    and len(probe.catalogs) == 1 and model.mcp_methods == ["tools/list"],
                    "late approval continued a cancelled Snow turn")
            process.call("session/load", {"sessionId": sid, "cwd": str(workspace), "mcpServers": []})
            result, frames = process.call("session/prompt", prompt)
            require(result["stopReason"] == "end_turn" and len(probe.catalogs) == 2
                    and PROJECTED_MCP not in probe.catalogs[1] and model.mcp_methods == ["tools/list"]
                    and all(row.get("method") != "session/request_permission" for row in frames),
                    "removed Snow MCP was reused after session/load")
            require(probe.failure is None, probe.failure)
        finally:
            process.close()


def verify(binary):
    machine = platform.machine()
    require(sys.platform == "linux" and machine in {"x86_64", "aarch64"}
            and os.geteuid() != 0 and os.confstr("CS_GNU_LIBC_VERSION"),
            "Snow acceptance requires a non-root Linux GNU x64 or ARM64 container")
    manifest = ROOT / "patches/acp-model-source/snow/linux.json"
    pin = json.loads(manifest.read_text())
    ripgrep = pin["ripgrep"][machine]
    with tempfile.TemporaryDirectory(prefix="shiju-snow-platform-") as directory:
        root = Path(directory).resolve()
        env = environment(root)
        # Contain the unpatched gateway's generic npm fallback during red runs.
        env["npm_config_prefix"] = str(root / "npm")
        env["npm_config_ignore_scripts"] = "true"
        runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
        with gateway(str(binary), root, env) as url:
            request(url, "/desktop/acp-install/snow", method="POST")
            spec = json.loads(request(url, "/global/config/acp/defaults"))["snow"]
            entry = runtime / "node_modules/snow-ai/bundle/cli.mjs"
            require(Path(spec["command"]) == runtime / "node_modules/.bin/snow"
                    and Path(spec["command"]).resolve(strict=True) == entry
                    and spec["args"] == ["--acp"], "gateway did not select reviewed Linux Snow")
            metadata = json.loads((entry.parents[1] / "package.json").read_text())
            require(metadata["name"] == "snow-ai" and metadata["version"] == "0.9.14",
                    "unexpected Snow package identity")
            require(hashlib.sha256(entry.read_bytes()).hexdigest() == pin["patched"],
                    "unexpected Snow entry digest")
            require(hashlib.sha256(entry.with_name("shiju-mcp.mjs").read_bytes()).hexdigest()
                    == pin["projection_sha256"], "unexpected Snow MCP projection digest")
            lock = json.loads((runtime / "package-lock.json").read_text())["packages"]
            helper = "node_modules/" + ripgrep["package"]
            require(lock[helper]["integrity"] == ripgrep["integrity"]
                    and lock[helper]["version"] == pin["ripgrep"]["version"]
                    and hashlib.sha256((runtime / helper / "bin/rg").read_bytes()).hexdigest()
                    == ripgrep["sha256"], "unexpected Snow native search helper")

        def launch(private_env):
            private = Path(private_env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
            shutil.copytree(runtime, private, symlinks=True)
            return {**spec, "command": str(private / "node_modules/.bin/snow")}, {
                "target": machine, "entry": "bundle/cli.mjs", "version": metadata["version"],
                "revision": REVISION, "sha256": pin["patched"],
                "ripgrep_sha256": ripgrep["sha256"],
                "model_endpoint": snow.API_PATH + "/chat/completions",
            }

        report = {"turns": []}
        with patch.multiple(snow, launch=launch, Probe=Probe, MCP_TOOL=PROJECTED_MCP,
                            mcp_server=mcp_server, create=True):
            run("snow", binary, report)
        require([row["mcp_calls"] for row in report["turns"]] == [0, 1, 0, 0],
                "Snow session MCP audit changed after rejection or removal")
        verify_cancel_and_load(entry, root, env)
        report["cancel_and_load"] = "passed"
        print(json.dumps(report))


if __name__ == "__main__":
    try:
        verify(Path(sys.argv[1]).resolve(strict=True))
    except AssertionError as error:
        sys.exit(str(error))
