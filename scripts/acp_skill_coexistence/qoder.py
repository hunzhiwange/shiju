"""Verify Qoder's native skill and stdio MCP tools through Chat Completions."""

from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess
import sys

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

from . import correlated_result

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
# The runner creates this private HOME next to its workspace.
NATIVE_DIR = "../home/.agents/skills"
VERSION = "1.1.88"
REVISION = f"qoder-{VERSION}-model-source-v2"
MCP_TOOL = "mcp__coexistence__read_probe"
MCP_TITLE = "read_probe (coexistence)"
MANAGED = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"


def launch(env):
    system, machine = platform.system(), platform.machine()
    linux_arch = {"x86_64": "x64", "aarch64": "arm64"}.get(machine)
    linux = system == "Linux" and linux_arch is not None and platform.libc_ver()[0] == "glibc"
    require(linux or (system, machine) == ("Darwin", "arm64"), "unreviewed Qoder platform")
    manifest = MANAGED / "contracts.json"
    patches = ROOT / "patches/acp-model-source/qoder"
    contract = (json.loads((patches / "linux.json").read_text()) if linux
                else json.loads(manifest.read_text())["qoder"])
    require(contract["version"] == VERSION, "unexpected Qoder contract version")
    revision = contract["revision"] if linux else REVISION
    target = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision
    package = target / "node_modules" / contract["package"]
    # The legacy Darwin fixture can reuse a private preinstalled package.
    supplied = os.environ.get("QODER_COEXISTENCE_PACKAGE")
    if supplied and not linux:
        source = Path(supplied).resolve(strict=True)
        require(all(hashlib.sha256((source / file).read_bytes()).hexdigest() == digest
                    for file, digest in contract["files"].items()), "Qoder artifact digest mismatch")
        shutil.copytree(source, package, symlinks=True)
    else:
        target.parent.mkdir()
        assets = Path(env["TMPDIR"]) / "qoder-assets"
        assets.mkdir(mode=0o700)
        shutil.copy2(manifest, assets)
        shutil.copytree(patches, assets / "qoder")
        try:
            result = subprocess.run(
                [sys.executable, str(MANAGED / "install.py"), str(assets), str(target), "qoder"],
                env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=180)
        except subprocess.TimeoutExpired as error:
            raise AssertionError("Qoder managed installation timed out") from error
        require(result.returncode == 0, "Qoder managed installation failed")
    hashes = {file: hashlib.sha256((package / file).read_bytes()).hexdigest()
              for file in contract["files"]}
    require(hashes == contract["files"], "Qoder artifact digest mismatch")
    command = package / contract["bin"]
    result = subprocess.run([str(command), "--version"], env=env,
                            capture_output=True, timeout=15)
    require(result.returncode == 0 and re.search(rb"\b1\.1\.88\b", result.stdout),
            "unexpected Qoder runtime version")
    artifact = {
        "platform": f"{system}-{machine}", "version": VERSION, "revision": revision, "sha256": hashes,
    }
    if linux:
        helper = contract["ripgrep"][linux_arch]
        rg = target / "node_modules" / helper["package"] / "bin/rg"
        digest = hashlib.sha256(rg.read_bytes()).hexdigest()
        require(digest == helper["sha256"], "Qoder ripgrep digest mismatch")
        artifact["ripgrep_sha256"] = digest
        artifact["model_protocol"] = "POST /v1/chat/completions"
    return {"command": str(command), "args": ["--acp"]}, artifact


@contextmanager
def mcp_server(workspace, attachment, audit):
    args = [str(ROOT / "scripts/acp_mount_integration.py"), "--mcp-stdio",
            str(attachment), str(audit)]
    env = []
    if platform.system() == "Linux":
        # Import only the test server code using the trusted repository path.
        args = ["-c", "import sys; sys.path.insert(0, sys.argv[1]); "
                "from acp_skill_coexistence.qoder import linux_mcp_stdio; "
                "linux_mcp_stdio(*sys.argv[2:])",
                str(ROOT / "scripts"), str(workspace / ".qoder-probe-call"),
                str(attachment), str(audit)]
        env = [{"name": "QODER_PROBE_" + name, "value": "${" + name + ":-absent}"}
               for name in ("SHIJU_QODER_MODEL_CONFIG", "VW_ACP_LAUNCH_REFERENCE")]
    yield {"name": "coexistence", "command": sys.executable, "args": args, "env": env}


def linux_mcp_stdio(expected_call, attachment, audit):
    from acp_mount_integration import rpc

    for name in ("SHIJU_QODER_MODEL_CONFIG", "VW_ACP_LAUNCH_REFERENCE"):
        require(name not in os.environ and os.environ.get("QODER_PROBE_" + name) == "absent",
                "Qoder exposed a binding variable to a tool subprocess")
    for line in sys.stdin:
        request = json.loads(line)
        if request.get("method") == "tools/call":
            call_id = request["params"].get("_meta", {}).get("shijuCallId")
            require(call_id == Path(expected_call).read_text(), "Qoder MCP invocation ID changed")
        response = rpc(request, attachment, audit, "stdio")
        if response is not None:
            print(json.dumps(response), flush=True)


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.failure = None
        self.pending = None

    def response(self, body):
        require(body["model"] == MODEL and body.get("stream") is True,
                "unexpected Qoder Chat Completions contract")
        tools = {row["function"]["name"]: row["function"] for row in body.get("tools", [])}
        self.requests += 1
        require({"Skill", "Read"} <= tools.keys(), "Qoder native Skill or Read tool is absent")
        require((MCP_TOOL in tools) == self.mounted, "unexpected Qoder stdio MCP catalog")
        require(not {"mcp_list", "mcp_get", "mcp_call"} & tools.keys(),
                "Qoder exposed lazy MCP tools")
        skills = self.fixtures["skills"]
        if self.requests == 1:
            envelope = json.dumps(body)
            names = {name for message in body["messages"] for block in self.text_blocks(message)
                     for catalog in re.findall(
                         r"The following skills are available[^\n]*\n(.*?)</system-reminder>",
                         block, re.S)
                     for name in re.findall(r"^- ([a-z0-9-]+): ", catalog, re.M)}
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in names) == visible, f"unexpected Qoder {kind} skill catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Qoder skill content appeared before activation")
            require(self.fixtures["mcp"] not in envelope, "Qoder MCP value appeared before invocation")
            return self.call(tools, "native", "Skill", {"skill": skills["native"]["name"]})

        step, _, call_id, _ = self.pending
        results = [dict(message, tool_use_id=message["tool_call_id"])
                   for message in body["messages"] if message["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        skill = skills.get(step.removesuffix("_read"))
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and re.search(r"reject|denied|cancel", result, re.I))
                    if self.rejected else value in result, "unexpected Qoder MCP continuation")
        else:
            if step.endswith("_read"):
                require(skill["attachment"] in result, f"missing Qoder {step} content")
            else:
                require("Launching skill: " + skill["name"] in result,
                        "unexpected Qoder activation result")
                result = self.activation(body["messages"], call_id, skill["body"])
        self.completed[step] = call_id

        if step in ("native", "selected"):
            match = re.search(r"Base directory for this skill: ([^\r\n]+)", result)
            require(match is not None, "Qoder skill activation omitted its directory")
            base = Path(match[1].strip())
            require(base.is_absolute() and base.name == skill["name"], "invalid Qoder skill directory")
            require(base.resolve() == skill["path"].resolve(), "unexpected Qoder skill directory")
            if step == "selected":
                require(base.parent.name == "skills" and base.parent.is_symlink()
                        and base.parent.resolve().parent.name.startswith("shiju-session-skills-"),
                        "Qoder selected skill bypassed the session view")
            return self.call(tools, step + "_read", "Read",
                             {"file_path": str(base / "references/value.txt")})
        if self.mounted and step == "native_read":
            return self.call(tools, "selected", "Skill", {"skill": skills["selected"]["name"]})
        if self.mounted and step == "selected_read":
            return self.call(tools, "mcp", MCP_TOOL, {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    @staticmethod
    def text_blocks(message):
        content = message.get("content")
        if isinstance(content, str):
            return [content]
        return [block["text"] for block in content or [] if block["type"] == "text"]

    def activation(self, messages, call_id, body):
        for index, message in enumerate(messages):
            if message.get("tool_call_id") != call_id:
                continue
            for following in messages[index + 1:]:
                if following["role"] != "user":
                    break
                for block in self.text_blocks(following):
                    if body in block:
                        return block
        raise AssertionError("Qoder skill body missing from correlated continuation")

    def call(self, tools, step, name, arguments):
        schema = tools[name]["parameters"]
        require(set(schema.get("required", ())) <= arguments.keys() <= schema.get("properties", {}).keys(),
                f"unexpected Qoder {step} tool schema")
        call_id = "call_" + secrets.token_hex(12)
        if step == "mcp" and platform.system() == "Linux":
            workspace = self.fixtures["skills"]["selected"]["path"].parents[2]
            (workspace / ".qoder-probe-call").write_text(call_id)
        self.pending = (step, name, call_id, arguments)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def expected_skill_approvals(self):
        return []

    def approves_skill(self, metadata):
        if self.pending is None:
            return None
        step, name, _, arguments = self.pending
        if (name == "Read" and metadata.get("arguments") == arguments
                and str(metadata.get("title", "")).startswith("Read ")):
            return step.removesuffix("_read")
        return None

    def approves(self, metadata):
        return (self.pending is not None and self.pending[:2] == ("mcp", MCP_TOOL)
                and metadata.get("title") == MCP_TITLE and metadata.get("arguments") == {})


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.path == "/v1/chat/completions", "unexpected Qoder model protocol")
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Qoder model request")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Qoder model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Qoder model envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.chunk(body["model"], dict(delta, role="assistant"), None)
        self.chunk(body["model"], {}, finish)
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()
