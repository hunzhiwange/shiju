"""Verify native Harn skill activation, attachment reads and scoped stdio MCP."""
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import secrets
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
REVISION = f"harn-{VERSION}-model-source-v5"
MCP_TOOL = "shiju_mcp_coexistence__read_probe"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Harn platform")
    contract_path = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(contract_path.read_text())["harn"]
    require(contract["version"] == VERSION, "unexpected Harn contract version")
    source = Path(os.environ.get("ACP_HARN_RUNTIME", Path.home() /
                  ".shiju-harness-dev/acp-runtime" / REVISION))
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    runtime.mkdir(parents=True, mode=0o700)
    files = contract["files"]
    receipt = json.loads((source / "build-receipt.json").read_text())
    require(all(receipt[key] == contract[key] for key in ("version", "source_sha256", "patch_sha256")),
            "unexpected Harn build receipt")
    for name, expected in {**files, "harn-native": receipt["engine_sha256"]}.items():
        target = runtime / name
        shutil.copyfile(source / name, target)
        checksum = hashlib.sha256()
        with target.open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                checksum.update(block)
        require(checksum.hexdigest() == expected, f"Harn {name} checksum mismatch")
        if name in ("harn", "harn-native"):
            target.chmod(0o700)
    (runtime / "build-receipt.json").write_text(json.dumps(receipt))
    result = subprocess.run([str(runtime / "harn-native"), "--version"], cwd=runtime,
                            env=env, capture_output=True, timeout=15)
    require(result.returncode == 0 and result.stdout.strip() == f"harn {VERSION}".encode(),
            "unexpected Harn version")
    return {"command": str(runtime / "harn"), "args": []}, {
        "revision": REVISION, "sha256": files, **receipt,
    }


def prepare_workspace(workspace):
    # Exercise Harn's own skill-dir substitution and use the resulting path
    # for the separate native attachment read.
    for root in (workspace / NATIVE_DIR, workspace / ".shiju/skills"):
        for manifest in root.glob("*/SKILL.md"):
            with manifest.open("a") as out:
                out.write("\nFixture directory: ${HARN_SKILL_DIR}\n")
    disabled = workspace / NATIVE_DIR / "disabled-probe"
    disabled.mkdir()
    (disabled / "SKILL.md").write_text(
        "---\nname: disabled-probe\ndescription: disabled fixture\n"
        "disable-model-invocation: true\n---\nDo not activate.\n")


class Probe:
    def __init__(self, fixtures, mounted, rejected, *, selected=None):
        self.fixtures = fixtures
        self.mounted = mounted
        self.selected = mounted if selected is None else selected
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.failure = None
        self.pending = None

    def response(self, body):
        self.requests += 1
        require(body["model"] == MODEL and body["stream"] is True,
                "unexpected Harn model protocol")
        tools = {tool["function"]["name"]: tool["function"] for tool in body["tools"]}
        require({"read_file", "write_file", "load_skill"} <= tools.keys(), "Harn omitted a native tool")
        require((MCP_TOOL in tools) == self.mounted, "unexpected Harn stdio MCP catalog")
        skills = self.fixtures["skills"]
        messages = body["messages"]
        if self.requests == 1:
            catalog = "\n".join(str(item.get("content")) for item in messages if item["role"] == "system")
            envelope = json.dumps(body)
            require("disabled-probe" not in catalog, "Harn advertised a model-disabled skill")
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.selected)
                require((skill["name"] in catalog) == visible, f"unexpected Harn {kind} catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        f"Harn disclosed {kind} contents before native activation")
            require(self.fixtures["mcp"] not in envelope, "Harn disclosed MCP content before invocation")
            return self.call(tools, "native", "load_skill", {"name": skills["native"]["name"]})

        if self.pending is None:
            return {"content": "COEXISTENCE_DONE"}, "stop"
        step, call_id = self.pending
        replies = [str(item["content"]) for item in messages
                   if item["role"] == "tool" and item.get("tool_call_id") == call_id]
        require(len(replies) == 1, f"missing Harn correlated {step} result")
        content = replies[0]
        if step in ("native", "selected"):
            require(skills[step]["body"] in content, f"Harn did not load {step} skill body")
            path = re.search(r'Fixture directory: ([^\r\n\\"]+)', content)
            require(path is not None, "Harn did not substitute the actual skill directory")
            location = Path(path[1].strip())
            require(location.is_absolute() and location.parent.name == step
                    and location.name == skills[step]["name"], "unexpected Harn snapshot location")
            self.completed[step] = call_id
            return self.call(tools, step + "_read", "read_file",
                             {"path": str(location / "references/value.txt")})
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in content and re.search(r"reject|denied|approved|cancel", content, re.I))
                    if self.rejected else value in content, "unexpected Harn MCP continuation")
        else:
            require(skills[step.removesuffix("_read")]["attachment"] in content,
                    f"Harn did not read {step} attachment")
        self.completed[step] = call_id
        if step == "native_read" and self.selected:
            return self.call(tools, "selected", "load_skill", {"name": skills["selected"]["name"]})
        if step == "selected_read" and self.mounted:
            return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        if self.rejected:
            return {"content": "The read-only MCP call was rejected and did not execute."}, "stop"
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def call(self, tools, step, name, args):
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= args.keys() <= schema.get("properties", {}).keys(),
                f"unexpected Harn tool schema for {step}")
        call_id = "call_harn_" + secrets.token_hex(12)
        self.pending = (step, call_id)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(args)}}]}, "tool_calls"

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == MCP_TOOL and metadata.get("arguments") == {})


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
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Harn model envelope")
            self.send_error(422)
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.chunk(body["model"], {"role": "assistant", **delta}, None)
        self.chunk(body["model"], {}, finish)
        self.wfile.write(b"data: [DONE]\n\n")


def native_control():
    """Compare CLI discovery with the ACP VM's binding on the same release."""
    with tempfile.TemporaryDirectory(prefix="shiju-harn-native-") as directory:
        root = Path(directory).resolve()
        env = environment(root)
        binary = Path(os.environ.get("ACP_HARN_NATIVE", Path.home() /
                      f".shiju-harness-dev/acp-runtime/harn-{VERSION}-model-source-v1/harn-native"))
        digest = hashlib.sha256(binary.read_bytes()).hexdigest()
        require(digest == "23c951212c045c56deea491f9ce9505b8298472daa862334b33413ea31dcdcca",
                "unexpected original Harn binary")
        artifact = {"version": VERSION, "entry_sha256": digest}
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
        skill_path = json.dumps(str(Path(env["HOME"]) / NATIVE_DIR / "native-probe"))
        pipeline.write_text('pipeline default(harness: Harness) {\n'
                            f'  const entry = {{name: "native-probe", skill_dir: {skill_path}}}\n'
                            '  const rendered = harness.agent.skill_render(entry, [], {})\n'
                            '  require rendered == "", "metadata-only rendering loaded the body"\n'
                            '  const result = harness.agent.load_skill("native-probe")\n'
                            '  require result != "", "missing body"\n'
                            '}\n')
        resolved = subprocess.run([str(binary), "skill", "resolved", "--from", str(pipeline), "--json"],
                                  cwd=workspace, env=env, capture_output=True, text=True, timeout=15)
        require(resolved.returncode == 0, "Harn CLI resolver failed")
        layers = {row["id"]: row["layer"] for row in map(json.loads, resolved.stdout.splitlines())}
        require(layers.get("native-probe") == "user" and layers.get("selected-probe") == "env",
                "Harn CLI did not resolve both fixture skill layers")
        script = workspace / "cli-probe.harn"
        script.write_text('fn main(harness: Harness) {\n'
                          '  for name in ["native-probe", "selected-probe"] {\n'
                          '    const body = harness.agent.load_skill(name)\n'
                          '    require contains(body, "Fixture body"), "missing CLI skill body"\n'
                          '  }\n'
                          '}\n')
        loaded = subprocess.run([str(binary), "run", str(script)], cwd=workspace, env=env,
                                capture_output=True, timeout=15)
        require(loaded.returncode == 0, "Harn CLI could not lazily load both fixture skills")
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
                        "cli_loaded_bodies": 2, "acp_metadata_only_render": "empty",
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
