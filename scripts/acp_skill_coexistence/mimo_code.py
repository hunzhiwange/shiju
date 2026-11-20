"""Exercise MiMo's native skill catalog and reader with the host's stdio MCP bridge."""
from contextlib import contextmanager
import ctypes
import hashlib
from inspect import getsource
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
from urllib.parse import unquote, urlsplit

from acp_mount_integration import rpc
from verify_acp_integration import require
from verify_acp_model_source import ROOT

from . import correlated_result
from .opencode import Handler, Probe as ChatProbe

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".agents/skills"
VERSION = "0.1.17"
REVISION = f"mimo-code-{VERSION}-model-source-v1"
MCP_TOOL = "coexistence_read_probe"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed MiMo platform")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"
                           / "contracts.json").read_text())["mimo-code"]
    require(contract["version"] == VERSION, "unexpected MiMo version contract")
    default = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    source = Path(os.environ.get("ACP_MIMO_REVIEWED_DIR", default / "acp-runtime" / REVISION))
    require((source / "build-receipt.json").is_file(), "reviewed MiMo build receipt unavailable")
    entry = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
    entry.mkdir(parents=True, mode=0o700)
    for name in ("mimo", "build-receipt.json"):
        shutil.copy2(source / name, entry / name)
    receipt = json.loads((entry / "build-receipt.json").read_text())
    require(all(receipt.get(key) == contract[key] for key in
                ("version", "source_sha256", "patch_sha256")), "MiMo build receipt mismatch")
    executable = entry / "mimo"
    require(hashlib.sha256(executable.read_bytes()).hexdigest() == receipt["sha256"],
            "MiMo executable digest mismatch")
    try:
        version = subprocess.run([str(executable), "--version"], env=env, capture_output=True,
                                 timeout=15)
    except subprocess.TimeoutExpired:
        raise AssertionError("MiMo version check timed out") from None
    require(version.returncode == 0 and version.stdout.strip().decode() == VERSION,
            "unexpected MiMo executable version")
    return {"command": str(executable), "args": ["acp"]}, {
        **{key: receipt[key] for key in ("version", "source_sha256", "patch_sha256", "sha256")},
        "revision": REVISION,
    }


@contextmanager
def mcp_server(workspace, attachment, _audit):
    # Stage the shared protocol handler where the stdio sandbox can read it.
    script = workspace / "mcp-fixture.py"
    script.write_text(
        "import json, os, sys\nfrom pathlib import Path\n"
        "def require(test, message):\n"
        "    if not test:\n        raise AssertionError(message)\n\n"
        + getsource(rpc)
        + "\naudit = Path(os.environ['TMPDIR']) / 'mimo-fixture-audit'\n"
        "audit.touch(exist_ok=False)\n"
        "for line in sys.stdin:\n"
        "    result = rpc(json.loads(line), sys.argv[1], audit, 'stdio')\n"
        "    if result is not None:\n        print(json.dumps(result), flush=True)\n")
    payload = workspace / "mcp-payload.txt"
    payload.write_text(attachment.read_text())
    # The CLT python3 command is a launcher. Use the running interpreter so
    # the stdio sandbox can keep child process execution disabled.
    executable = ctypes.create_string_buffer(4096)
    libproc = ctypes.CDLL("/usr/lib/libproc.dylib")
    require(libproc.proc_pidpath(os.getpid(), executable, len(executable)) > 0,
            "could not resolve the fixture interpreter")
    yield {"name": "coexistence", "command": os.fsdecode(executable.value),
           "args": ["-I", "-B", str(script), str(payload)], "env": []}


class Probe(ChatProbe):
    def response(self, body):
        require(body["model"] == MODEL, "unexpected MiMo model")
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        # Auxiliary classification requests can advertise StructuredOutput.
        if "read" not in tools:
            return {"content": "MiMo fixture session"}, "stop"
        self.requests += 1
        skills = self.fixtures["skills"]
        messages = body["messages"]
        if self.requests == 1:
            catalog = "\n".join(str(row["content"]) for row in messages if row["role"] == "system")
            listed = dict(re.findall(r"<skill>\s*<name>([^<]+)</name>\s*"
                                     r"<description>[^<]*</description>\s*"
                                     r"<location>([^<]+)</location>\s*</skill>", catalog))
            require((MCP_TOOL in tools) == self.mounted, "unexpected MiMo MCP inventory")
            self.paths = {}
            envelope = json.dumps(body)
            for kind, skill in skills.items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in listed) == visible, f"unexpected MiMo {kind} catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "MiMo included skill content before activation")
                if visible:
                    uri = urlsplit(listed[skill["name"]])
                    path = Path(unquote(uri.path))
                    require(uri.scheme == "file" and not uri.netloc and path.is_absolute()
                            and path.resolve() == (skill["path"] / "SKILL.md").resolve(),
                            f"MiMo advertised the wrong {kind} skill path")
                    if kind == "selected":
                        require(path.parent.parent.name == "skills"
                                and path.parent.parent.parent.name.startswith("shiju-session-skills-"),
                                "MiMo selected skill bypassed the trusted view")
                    self.paths[kind] = path
            require(self.fixtures["mcp"] not in envelope, "MiMo included MCP data before invocation")
            if self.mounted:
                # Keep the real audit inode after the bridge removes its scratch directory.
                root = skills["native"]["path"].parents[3]
                audits = list((root / "tmp").glob("shiju-mimo-mcp-*/mimo-fixture-audit"))
                require(len(audits) == 1, "missing or ambiguous MiMo stdio audit")
                self.audit = root / ("mimo-rejected-audit" if self.rejected else "mimo-mounted-audit")
                try:
                    os.link(audits[0], self.audit)
                except OSError:
                    raise AssertionError("could not retain the MiMo stdio audit") from None
                require(not self.audit.read_text(), "MiMo called MCP before the model requested it")
            return self.read(tools, "native", self.paths["native"])

        step, _, _, call_id = self.pending
        results = [dict(row, tool_use_id=row["tool_call_id"])
                   for row in messages if row["role"] == "tool"]
        result = str(correlated_result(results, call_id)["content"])
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in result and re.search(r"reject|denied|cancel", result, re.I))
                    if self.rejected else value in result, "unexpected MiMo MCP continuation")
        else:
            kind = step.removesuffix("_read")
            skill = skills[kind]
            expected = skill["body"] if step == kind else skill["attachment"]
            require(expected in result, f"MiMo did not read the {step} content")
            if step == kind:
                require(skill["attachment"] not in result, "MiMo attached data before its read")
        self.completed[step] = call_id
        if step in self.paths:
            return self.read(tools, step + "_read", self.paths[step].parent / "references/value.txt")
        if self.mounted and step == "native_read":
            return self.read(tools, "selected", self.paths["selected"])
        if self.mounted and step == "selected_read":
            delta, finish = self.call(tools, "mcp", MCP_TOOL, {})
            if self.rejected:
                # MiMo can end the prompt immediately on refusal without a final model request.
                delta["content"] = "Checking the read-only MCP fixture."
            return delta, finish
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def read(self, tools, step, path):
        return self.call(tools, step, "read", {"file_path": str(path)})

    def observe_terminal(self, events):
        if not self.rejected or self.pending is None:
            return False
        # MiMo can replay the same terminal update on its ACP event stream.
        terminal = {}
        for event in events:
            if (event.get("type") == "chat.tool" and event.get("kind") == "complete"
                    and event.get("call_id") == self.pending[3]):
                payload = event["payload"]
                key = (event.get("status"), payload.get("title"), payload.get("error"))
                terminal[key] = event
        return super().observe_terminal(terminal.values())

    def mcp_trace(self, _events):
        audit = self.audit.read_text().splitlines() if self.mounted else []
        return audit, self.completed.get("mcp")
