"""Exercise the signed Junie ACP and its confined HTTP-to-stdio bridge."""

from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import secrets
import shutil
import subprocess

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

from .cortex_code import mcp_server as confined_stdio_fixture

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".junie/skills"
VERSION = "26.11.2 (4100.5)"
MCP_ARGS = {"source": "fixture"}
MANAGED = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"
PATCH = ROOT / "patches/acp-model-source"


def digest(path):
    checksum = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(1024 * 1024), b""):
            checksum.update(block)
    return checksum.hexdigest()


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Junie platform")
    contract = json.loads((MANAGED / "contracts.json").read_text())["junie"]
    require(contract["version"] == "4100.5", "unreviewed Junie version")
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / "junie-4100.5-model-source-v3"
    runtime.parent.mkdir()
    archive = os.environ.get("ACP_JUNIE_ARCHIVE")
    if archive:
        archive = Path(archive).resolve(strict=True)
        require(digest(archive)
                == contract["archive_sha256"], "Junie archive digest mismatch")
        subprocess.run(["/usr/bin/ditto", "-x", "-k", str(archive), str(runtime)],
                       env=env, capture_output=True, check=True, timeout=60)
        for name, source in {"junie": PATCH / "junie/launcher.py",
                             "mcp_bridge.py": PATCH / "junie/mcp_bridge.py",
                             "mcp_stdio.py": PATCH / "cortex-code/mcp_bridge.py"}.items():
            shutil.copy2(source, runtime / name)
        (runtime / "junie").chmod(0o700)
    else:
        assets = Path(env["TMPDIR"]) / "junie-assets"
        assets.mkdir(mode=0o700)
        shutil.copy2(MANAGED / "contracts.json", assets)
        (assets / "cortex-code").mkdir()
        shutil.copy2(PATCH / "cortex-code/mcp_bridge.py", assets / "cortex-code/mcp_bridge.py")
        shutil.copytree(PATCH / "junie", assets / "junie")
        result = subprocess.run(["/usr/bin/python3", str(MANAGED / "install.py"),
                                 str(assets), str(runtime), "junie"],
                                env=env, capture_output=True, timeout=160)
        require(result.returncode == 0, "Junie reviewed installation failed")
    hashes = {}
    for name, expected in contract["files"].items():
        hashes[name] = digest(runtime / name)
        require(hashes[name] == expected, f"Junie digest mismatch for {name}")
    command = runtime / "junie"
    version = subprocess.run([str(command), "--version"], env=env, cwd=runtime,
                             capture_output=True, timeout=20)
    require(version.returncode == 0 and version.stdout.strip() == f"Junie version: {VERSION}".encode(),
            "Junie signing or version check failed")
    return {"command": str(command), "args": ["--acp=true"]}, {
        "revision": runtime.name, "version": VERSION, "archive_sha256": contract["archive_sha256"],
        "sha256": hashes,
    }


@contextmanager
def mcp_server(workspace, attachment, audit):
    # Junie supplies an approval payload only when the tool has arguments.
    # Keep the same confined reader and independent FIFO audit as Cortex.
    with confined_stdio_fixture(workspace, attachment, audit) as service:
        args = service["args"]
        fixture = Path(args[1]).with_name("junie_stdio.py")
        fixture.write_text('''import json
import sys
from acp_mount_integration import rpc

for line in sys.stdin:
    request = json.loads(line)
    if request["method"] == "tools/call" and request["params"].get("arguments") != {"source": "fixture"}:
        raise ValueError("Unexpected Junie probe arguments")
    response = rpc(request, sys.argv[1], sys.argv[2], "stdio")
    if response and request["method"] == "tools/list":
        response["result"]["tools"][0]["inputSchema"] = {
            "type": "object", "properties": {"source": {"type": "string"}},
            "required": ["source"], "additionalProperties": False,
        }
    if response:
        print(json.dumps(response), flush=True)
''')
        yield {**service, "args": [args[0], str(fixture), *args[3:]]}


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.path == "/v1/chat/completions", "unexpected Junie model endpoint")
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated Junie model request")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid Junie model request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, TypeError, ValueError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid Junie model envelope")
            self.send_error(422)
            return
        message = dict(delta, role="assistant")
        if body.get("stream"):
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            self.chunk(body["model"], message, None)
            self.chunk(body["model"], {}, finish)
            self.wfile.write(b"data: [DONE]\n\n")
        else:
            for call in message.get("tool_calls", []):
                call.pop("index", None)
            content = json.dumps({
                "id": "chatcmpl-junie-probe", "object": "chat.completion", "created": 1,
                "model": body["model"], "choices": [{"index": 0, "message": message,
                                                     "finish_reason": finish}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
            }).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(content)))
            self.end_headers()
            self.wfile.write(content)


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
        require(body["model"] == MODEL, "unexpected Junie model alias")
        self.requests += 1
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if "open" not in tools:
            return {"content": "Fixture skill check"}, "stop"
        skills = self.fixtures["skills"]
        if self.pending is None:
            envelope = json.dumps(body)
            require("agent_skill_read_doc" in tools, "Junie omitted its native skill reader")
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        f"Junie disclosed {kind} contents before activation")
                if (skill["name"] in envelope) != visible:
                    self.failure = self.failure or f"unexpected Junie {kind} skill catalog"
            require(self.fixtures["mcp"] not in envelope, "Junie disclosed the MCP value before dispatch")
            if not self.mounted:
                require(not self.mcp_tools(tools) and "enable_mcp" not in tools,
                        "unexpected Junie MCP discovery outside mounted phase")
            if self.failure:
                # Let the old gateway finish the real baseline turn.
                return self.answer(tools)
            return self.read(tools, "native")

        step, call_id = self.pending
        results = [row["content"] for row in body["messages"] if row["role"] == "tool"
                   and row.get("tool_call_id") == call_id]
        require(len(results) == 1, f"missing or ambiguous Junie {step} continuation")
        content = str(results[0])
        if step == "enable_mcp":
            return self.read_mcp(tools)
        if step == "mcp":
            value = self.fixtures["mcp"]
            if self.rejected:
                require(value not in content and any(word in content.lower()
                        for word in ("denied", "reject", "declin", "cancel")),
                        "Junie MCP rejection did not reach the model")
            else:
                require(value in content, "missing Junie MCP result")
        else:
            kind = step.removesuffix("_read")
            field = "attachment" if step.endswith("_read") else "body"
            require(skills[kind][field] in content, f"missing Junie {step} contents")
        self.completed[step] = call_id

        if step in ("native", "selected"):
            return self.read(tools, step, path="references/value.txt")
        if step == "native_read" and self.mounted:
            return self.read(tools, "selected")
        if step == "selected_read":
            if self.mcp_tools(tools):
                return self.read_mcp(tools)
            require("enable_mcp" in tools, "Junie omitted its native MCP activation tool")
            schema = tools["enable_mcp"]["parameters"]
            field, = schema["required"]
            value = "coexistence" if schema["properties"][field].get("type") == "string" else ["coexistence"]
            return self.call(tools, "enable_mcp", "enable_mcp", {field: value})
        return self.answer(tools)

    def answer(self, tools):
        return self.call(tools, "answer", "answer",
                         {"full_answer": "Fixture skill check finished.", "is_terminal": True})

    def read(self, tools, kind, *, path=None):
        arguments = {"name": self.fixtures["skills"][kind]["name"]}
        if path is not None:
            arguments["path"] = path
        return self.call(tools, kind if path is None else kind + "_read", "agent_skill_read_doc", arguments)

    @staticmethod
    def mcp_tools(tools):
        return [name for name in tools if name.endswith("read_probe") and "coexistence" in name]

    def read_mcp(self, tools):
        names = self.mcp_tools(tools)
        require(len(names) == 1, "Junie did not expose the selected read-only MCP tool")
        return self.call(tools, "mcp", names[0], MCP_ARGS)

    def call(self, tools, step, name, arguments):
        require(name in tools, f"Junie omitted the tool for {step}")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= arguments.keys() <= schema["properties"].keys(),
                f"unexpected Junie tool schema for {step}")
        call_id = "call_junie_" + secrets.token_hex(12)
        self.pending = (step, call_id)
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == "coexistence/read_probe"
                and metadata.get("arguments") in (None, MCP_ARGS))
