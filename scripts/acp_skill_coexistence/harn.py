"""Locate the first native skill coexistence failure in the reviewed Harn ACP."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import time

from verify_acp_approval import environment
from verify_acp_integration import Process, ProviderHandler, require
from verify_acp_model_source import ROOT

from .cortex_code import mcp_server as mcp_server

MODEL = "deepseek-v4.1-flash-1017"
MODEL_OUTPUT_LIMIT = 16384
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".harn/skills"
VERSION = "0.10.201"
REVISION = f"harn-{VERSION}-model-source-v4"
MCP_TOOL = "shiju_mcp_coexistence__read_probe"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Harn platform")
    contract_path = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(contract_path.read_text())["harn"]
    require(contract["version"] == VERSION, "unexpected Harn contract version")
    # Older managed installs can supply the same signed release binary, while
    # this probe stages only the pinned v4 artifacts in the isolated account.
    native = Path(os.environ.get("ACP_HARN_NATIVE", Path.home() /
                  f".shiju-harness-dev/acp-runtime/harn-{VERSION}-model-source-v1/harn-native"))
    patches = ROOT / "patches/acp-model-source"
    assets = {"harn-native": native, "agent.harn": patches / "harn/agent.harn",
              "harn": patches / "harn/launcher.py",
              "shiju_stdio.py": patches / "cortex-code/mcp_bridge.py"}
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    runtime.mkdir(parents=True, mode=0o700)
    require(assets.keys() == contract["files"].keys(), "unexpected Harn artifact manifest")
    for name, source in assets.items():
        target = runtime / name
        shutil.copyfile(source, target)
        checksum = hashlib.sha256()
        with target.open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                checksum.update(block)
        digest = checksum.hexdigest()
        require(digest == contract["files"][name], f"Harn {name} checksum mismatch")
        if name in ("harn", "harn-native"):
            target.chmod(0o700)
    result = subprocess.run([str(runtime / "harn-native"), "--version"], cwd=runtime,
                            env=env, capture_output=True, timeout=15)
    require(result.returncode == 0 and result.stdout.strip() == f"harn {VERSION}".encode(),
            "unexpected Harn version")
    return {"command": str(runtime / "harn"), "args": []}, {
        "version": VERSION, "revision": REVISION, "sha256": contract["files"],
    }


class Probe:
    def __init__(self, fixtures, mounted, _rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.requests = 0
        self.completed = {}
        self.failure = None

    def response(self, body):
        self.requests += 1
        require(body["model"] == MODEL and body["stream"] is True,
                "unexpected Harn model protocol")
        tools = {tool["function"]["name"] for tool in body["tools"]}
        require({"read_file", "write_file"} <= tools, "Harn omitted the native file tools")
        require((MCP_TOOL in tools) == self.mounted, "unexpected Harn stdio MCP catalog")
        envelope = json.dumps(body)
        for kind, skill in self.fixtures["skills"].items():
            require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                    f"Harn disclosed {kind} contents before native activation")
        require(self.fixtures["mcp"] not in envelope, "Harn disclosed MCP content before invocation")
        native = self.fixtures["skills"]["native"]["name"]
        if "load_skill" not in tools or native not in envelope:
            self.failure = "Harn ACP omitted the native skill catalog and lazy loader"
        else:
            self.failure = "Harn native activation and mounted continuation require requalification"
        # Finish the real baseline turn so gateway/model availability is
        # distinguishable from the missing skill registry.
        return "HARN_BASELINE_REACHED"

    def approves(self, _metadata):
        return False


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.path == "/v1/chat/completions", "unexpected Harn model endpoint")
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Harn model request")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 2_000_000, "unexpected Harn model request size")
            body = json.loads(self.rfile.read(size))
            content = probe.response(body)
        except (AssertionError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Harn model envelope")
            self.send_error(422)
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.chunk(body["model"], {"role": "assistant", "content": content}, None)
        self.chunk(body["model"], {}, "stop")
        self.wfile.write(b"data: [DONE]\n\n")


def native_control():
    """Compare CLI discovery with the ACP VM's binding on the same release."""
    with tempfile.TemporaryDirectory(prefix="shiju-harn-native-") as directory:
        root = Path(directory).resolve()
        env = environment(root)
        spec, artifact = launch(env)
        binary = Path(spec["command"]).with_name("harn-native")
        workspace = root / "workspace"
        workspace.mkdir()
        env_root = root / "selected"
        env["HARN_SKILLS_PATH"] = str(env_root)
        for parent, name in ((Path(env["HOME"]) / NATIVE_DIR, "native-probe"),
                             (env_root, "selected-probe")):
            skill = parent / name
            skill.mkdir(parents=True)
            (skill / "SKILL.md").write_text(
                f"---\nname: {name}\ndescription: Private fixture\n---\nFixture body\n")
        pipeline = workspace / "probe.harn"
        pipeline.write_text('pipeline default(harness: Harness) {\n'
                            '  const result = harness.agent.load_skill("native-probe")\n'
                            '  require result != "", "missing body"\n'
                            '}\n')
        resolved = subprocess.run([str(binary), "skill", "resolved", "--from", str(pipeline), "--json"],
                                  cwd=workspace, env=env, capture_output=True, text=True, timeout=15)
        require(resolved.returncode == 0, "Harn CLI resolver failed")
        layers = {row["id"]: row["layer"] for row in map(json.loads, resolved.stdout.splitlines())}
        require(layers.get("native-probe") == "user" and layers.get("selected-probe") == "env",
                "Harn CLI did not resolve both fixture skill layers")
        process = Process([str(binary), "serve", "acp", "--session-environment", "isolated", str(pipeline)],
                          workspace, env, root / "native.stderr")
        try:
            process.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
            session, _ = process.call("session/new", {"cwd": str(workspace), "mcpServers": []})
            request = process.send("session/prompt", {"sessionId": session["sessionId"],
                                   "prompt": [{"type": "text", "text": "Read the private fixture."}]})
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                frame = process.frames.get(timeout=max(0.01, deadline - time.monotonic()))
                require("eof" not in frame and "invalid_stdout" not in frame, "invalid Harn ACP stream")
                if frame.get("id") == request and "method" not in frame:
                    failure = "no skill registry is bound to this scope"
                    require(failure in json.dumps(frame.get("error")), "unexpected Harn ACP loader outcome")
                    return {"artifact": artifact, "cli_layers": {
                        "native": layers["native-probe"], "selected": layers["selected-probe"]},
                        "acp_call": "harness.agent.load_skill", "acp_error": failure}
                if "id" in frame and "method" in frame:
                    response = {"jsonrpc": "2.0", "id": frame["id"],
                                "error": {"code": -32601, "message": "Host capability is unavailable"}}
                    process.child.stdin.write(json.dumps(response) + "\n")
                    process.child.stdin.flush()
            raise AssertionError("Harn ACP loader diagnostic timed out")
        finally:
            process.close()


if __name__ == "__main__":
    print(json.dumps(native_control(), indent=2))
