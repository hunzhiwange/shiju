"""Locate VT Code's managed ACP skill discovery boundary."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile

from verify_acp_integration import Process, ProviderHandler, require
from verify_acp_model_source import ROOT

from . import correlated_result
from .cortex_code import mcp_server as mcp_server
from .opencode import Probe as ChatProbe

VERSION = "0.183.0"
REVISION = f"vtcode-{VERSION}-model-source-v3"
MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"  # The managed gateway projects host stdio onto private HTTP.
NATIVE_DIR = ".agents/skills"
LOAD = "load_skill"
READ = "load_skill_resource"
MCP = "mcp__coexistence__read_probe"


def checksum(path):
    sha256 = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            sha256.update(block)
    return sha256.hexdigest()


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed VT Code platform")
    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(manifest.read_text())["vtcode"]
    require(contract["version"] == VERSION, "unexpected VT Code contract version")
    name = contract["bin"]
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = Path(os.environ.get("ACP_VTCODE_REVIEWED_BIN", base / "acp-runtime" / REVISION / name))
    require(source.is_file(), "reviewed VT Code patched executable unavailable")
    entry = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION / name
    entry.parent.mkdir(parents=True, mode=0o700)
    shutil.copy2(source, entry)
    digest = checksum(entry)
    require(digest == contract["files"][name], "VT Code patched executable digest mismatch")
    try:
        version = subprocess.run([str(entry), "--version"], env=env,
                                 capture_output=True, timeout=15)
    except subprocess.TimeoutExpired:
        raise AssertionError("VT Code executable version check timed out") from None
    require(version.returncode == 0 and version.stdout.splitlines()[:1] == [b"vtcode-core 0.183.0"], "unexpected VT Code executable version")
    return {"command": str(entry), "args": ["acp"]}, {
        "version": VERSION, "revision": REVISION, "platform": "darwin-arm64", "sha256": digest,
    }


class Probe(ChatProbe):
    def response(self, body):
        require(body["model"] == MODEL, "unexpected VT Code model")
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if not tools:
            return {"content": "Fixture session"}, "stop"
        self.requests += 1
        skills = self.fixtures["skills"]
        if self.requests == 1:
            envelope = json.dumps(body)
            require({LOAD, READ} <= tools.keys(), "VT Code ACP read-only skills are unavailable")
            catalog = tools[LOAD]["description"]
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog) == visible, f"unexpected VT Code {kind} catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "VT Code exposed skill content before activation")
            require(self.fixtures["mcp"] not in envelope, "VT Code exposed MCP data before invocation")
            require((MCP in tools) == self.mounted, "unexpected VT Code MCP catalog")
            require(tools.keys() <= {"read_file", "list_files", "code_search", "apply_patch", LOAD, READ, MCP},
                    "VT Code relay tool scope changed")
            return self.activate(tools, "native")

        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"])
                   for row in body["messages"] if row["role"] == "tool"]
        content = str(correlated_result(results, call_id)["content"])
        self.completed[step] = call_id
        if step in ("native", "selected"):
            skill = skills[step]
            require(skill["body"] in content and skill["attachment"] not in content,
                    f"unexpected VT Code {step} activation result")
            return self.call(tools, step + "_read", READ,
                             {"skill_name": skill["name"], "resource_path": "references/value.txt"})
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in content and "permission denied" in content.lower())
                    if self.rejected else value in content, "unexpected VT Code MCP continuation")
        else:
            require(skills[step.removesuffix("_read")]["attachment"] in content,
                    f"VT Code {step} did not read the fixture attachment")
        if self.mounted and step == "native_read":
            return self.activate(tools, "selected")
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", MCP, {})
        self.pending = None
        return {"content": "VTCODE_COEXISTENCE_DONE"}, "stop"

    def activate(self, tools, kind):
        return self.call(tools, kind, LOAD, {"name": self.fixtures["skills"][kind]["name"]})

    def observe_terminal(self, _events):
        return False


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated VT Code model request")
            require(self.path == "/v1/chat/completions", "unexpected VT Code model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid VT Code model request size")
            body = json.loads(self.rfile.read(size))
            require(body.get("stream") is False, "unexpected VT Code model streaming mode")
            message, finish = probe.response(body)
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid VT Code Chat Completions envelope")
            self.send_error(422, "probe rejected model request")
            return
        data = json.dumps({"id": "chatcmpl-fixture", "object": "chat.completion",
            "model": body["model"], "choices": [{"index": 0, "finish_reason": finish,
            "message": dict(message, role="assistant")}]}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)


def native_control(original):
    """Compare the CLI skill scanner with both actual ACP targets before the ShiJu relay."""
    from verify_acp_approval import environment
    from verify_acp_skill_coexistence import make_skill, model_server

    class Catalog:
        failure = None

        def __init__(self):
            self.requests = []

        def response(self, body):
            require(body["model"] == MODEL, "unexpected native control model")
            tools = sorted(item["function"]["name"] for item in body.get("tools", []))
            system = json.dumps([row for row in body["messages"] if row["role"] == "system"])
            envelope = json.dumps(body)
            self.requests.append({
                "tools": tools,
                "skill_metadata": [kind for kind, skill in fixtures.items() if skill["name"] in system],
                "skill_content": [kind for kind, skill in fixtures.items()
                                  if skill["body"] in envelope or skill["attachment"] in envelope],
            })
            return {"content": "VTCODE_NATIVE_CONTROL_DONE"}, "stop"

    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    digest = json.loads(manifest.read_text())["vtcode"]["source_sha256"]
    with tempfile.TemporaryDirectory(prefix="shiju-vtcode-native-") as directory:
        root = Path(directory).resolve()
        env = environment(root)
        patched, patched_receipt = launch(env)
        original_entry = root / "vtcode-original"
        shutil.copy2(original, original_entry)
        require(checksum(original_entry) == digest, "VT Code original executable digest mismatch")
        workspace = root / "work"
        workspace.mkdir()
        home = Path(env["HOME"])
        fixtures = {
            "project": make_skill(workspace / NATIVE_DIR, "project"),
            "private_home": make_skill(home / NATIVE_DIR, "private-home"),
        }
        key = os.urandom(24).hex()
        env.update(VT_ACP_ENABLED="1", VT_ACP_ZED_ENABLED="1", ACP_COEXISTENCE_KEY=key)
        config = home / "vtcode.toml"
        policy = home / "control.sb"
        with model_server(Handler, key) as model:
            port = model.server_port
            config.write_text(f"""[agent]
provider = "local-provider"
default_model = "{MODEL}"
reasoning_effort = "none"
[[custom_providers]]
name = "local-provider"
display_name = "Private model fixture"
base_url = "http://127.0.0.1:{port}/v1"
api_key_env = "ACP_COEXISTENCE_KEY"
model = "{MODEL}"
models = ["{MODEL}"]
api_format = "openai-chat"
supports_tools = true
[skills.bundled]
enabled = false
""")
            results = []
            for artifact, executable, sha256 in (
                ("original", original_entry, digest),
                ("patched", Path(patched["command"]), patched_receipt["sha256"]),
            ):
                policy.write_text(f"""(version 1)
(allow default)
(deny file-read-data (subpath "/Users"))
(allow file-read-data (subpath {json.dumps(str(root))}))
(deny file-write*)
(allow file-write* (subpath {json.dumps(str(home))}) (literal "/dev/null"))
(deny process-fork process-exec)
(allow process-exec (literal {json.dumps(str(executable))}))
(deny network*)
(allow network-outbound (remote ip "localhost:{port}"))
""")
                command = ["/usr/bin/sandbox-exec", "-f", str(policy),
                           str(executable), "--config", str(config)]
                names = {kind: skill["name"].encode() for kind, skill in fixtures.items()}
                scan = subprocess.run([*command, "skills", "list"], cwd=workspace, env=env,
                                      capture_output=True, timeout=15)
                require(scan.returncode == 0 and all(name in scan.stdout for name in names.values()),
                        f"VT Code {artifact} CLI scanner control failed")
                for target in ("zed", "standard"):
                    model.probe = probe = Catalog()
                    process = Process([*command, "acp", target], workspace, env,
                                      home / f"{artifact}-{target}.log")
                    try:
                        process.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
                        session, updates = process.call("session/new", {
                            "cwd": str(workspace), "mcpServers": [],
                        })
                        for prompt in ("Check available fixture skills.", "/skills list",
                                       "$" + fixtures["project"]["name"]):
                            turn, events = process.call("session/prompt", {
                                "sessionId": session["sessionId"],
                                "prompt": [{"type": "text", "text": prompt}],
                            })
                            require(turn["stopReason"] == "end_turn", "native control did not finish")
                            updates.extend(events)
                        require(not probe.failure and len(probe.requests) == 3,
                                "native control did not reach the model for each ACP prompt")
                        command_updates = [
                            frame["params"]["update"].get("availableCommands", [])
                            for frame in updates if frame.get("method") == "session/update"
                        ]
                        commands = {item["name"] for group in command_updates for item in group}
                        results.append({
                            "artifact": artifact, "sha256": sha256, "target": target,
                            "cli_discovered": sorted(names), "requests": probe.requests,
                            "advertised_fixture_commands": [kind for kind, skill in fixtures.items()
                                                            if skill["name"] in commands],
                            "advertised_skills_command": "skills" in commands,
                        })
                    finally:
                        process.close()
            return results


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Compare the pinned CLI and native ACP skill catalogs")
    parser.add_argument("--original", required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(native_control(args.original), indent=2))
