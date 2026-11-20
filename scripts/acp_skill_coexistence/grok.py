"""Probe Grok Build's native skills and read-only stdio MCP."""

import hashlib
import json
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess

from verify_acp_integration import require

from .cursor import Handler as Handler

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".grok/skills"
VERSION = "1.0.77 (50ca579fa996)"
SHA256 = "197c66a63e5ac46e07450b0ca21d141ce665050a412180f185dadfae1c7a5b2a"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed Grok Build platform")
    original = Path.home() / ".grok/bin/grok-1.0.77"
    entry = Path(env["HOME"]) / "grok"
    shutil.copy2(original, entry)
    checksum = hashlib.sha256()
    with entry.open("rb") as binary:
        for block in iter(lambda: binary.read(1024 * 1024), b""):
            checksum.update(block)
    digest = checksum.hexdigest()
    require(digest == SHA256, "unexpected Grok Build artifact digest")
    result = subprocess.run([str(entry), "--version"], env=env, capture_output=True, timeout=15)
    require(result.returncode == 0 and VERSION.encode() in result.stdout,
            "unexpected Grok Build version")
    return {"command": str(entry), "args": ["agent", "stdio"]}, {
        "version": VERSION, "entry_sha256": digest,
    }


def prepare_workspace(workspace):
    # Grok's native user scanner can discover the fixture without trusting the
    # project. The actual files stay under the checkout so ACP reads stay scoped.
    compatibility = workspace.parent / "home" / ".claude"
    compatibility.mkdir()
    (compatibility / "skills").symlink_to(workspace / NATIVE_DIR, target_is_directory=True)


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.completed = {}
        self.failure = None
        self.pending = None
        self.locations = {}

    def response(self, body):
        require(body["model"] == MODEL, "unexpected Grok model")
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        if "read_file" not in tools:
            return {"content": "Fixture session"}, "stop"
        self.requests += 1
        skills = self.fixtures["skills"]
        messages = body["messages"]
        if self.requests == 1:
            catalog = "\n".join(str(row["content"]) for row in messages
                                if row["role"] in ("system", "user"))
            envelope = json.dumps(body)
            for kind, skill in skills.items():
                entries = re.findall(
                    rf"^- {re.escape(skill['name'])}: [^\n]+\n  Absolute path: ([^\n]+)$",
                    catalog, re.M,
                )
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog) == visible and len(entries) == int(visible),
                        f"unexpected Grok {kind} skill catalog")
                require(skill["body"] not in envelope and skill["attachment"] not in envelope,
                        "Grok supplied skill contents before reading")
                if entries:
                    file = Path(entries[0])
                    require(file.is_absolute() and file.name == "SKILL.md",
                            "unexpected Grok skill catalog path")
                    require(file.resolve() == (skill["path"] / "SKILL.md").resolve(),
                            f"Grok changed the {kind} skill source")
                    if kind == "native":
                        file = file.resolve()
                    else:
                        require(any(part.startswith("shiju-session-skills-") for part in file.parts),
                                "Grok selected skill bypassed its session view")
                    self.locations[kind] = file
            require(self.fixtures["mcp"] not in envelope, "Grok disclosed MCP value before invocation")
            require(("read_probe" in tools) == self.mounted, "unexpected Grok stdio MCP catalog")
            return self.read(tools, "native", self.locations["native"])

        step, call_id = self.pending
        results = [row["content"] for row in messages if row["role"] == "tool"
                   and row["tool_call_id"] == call_id]
        require(len(results) == 1, "missing or ambiguous Grok tool continuation")
        content = str(results[0])
        if step == "mcp":
            value = self.fixtures["mcp"]
            require((value not in content and re.search(r"reject|denied|cancel", content, re.I))
                    if self.rejected else value in content, "unexpected Grok MCP continuation")
        else:
            skill = skills[step.removesuffix("_read")]
            field = "attachment" if step.endswith("_read") else "body"
            require(skill[field] in content, f"Grok did not return {step} fixture contents")
        self.completed[step] = call_id

        if step in self.locations:
            return self.read(tools, step + "_read", self.locations[step].parent / "references/value.txt")
        if step == "native_read" and self.mounted:
            return self.read(tools, "selected", self.locations["selected"])
        if step == "selected_read":
            return self.call(tools, "mcp", "read_probe", {})
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def read(self, tools, step, path):
        return self.call(tools, step, "read_file", {"target_file": str(path)})

    def call(self, tools, step, name, args):
        require(name in tools, f"Grok omitted tool for {step}")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= args.keys() <= schema.get("properties", {}).keys(),
                f"unexpected Grok tool schema for {step}")
        call_id = "call_grok_" + secrets.token_hex(12)
        self.pending = (step, call_id)
        delta = {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                 "function": {"name": name, "arguments": json.dumps(args)}}]}
        if step == "mcp":
            # Grok can end the turn at refusal before any model continuation.
            delta["content"] = "I will request the read-only fixture value."
        return delta, "tool_calls"

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == "coexistence__read_probe"
                and metadata.get("arguments") == {
                    "variant": "UseTool", "tool_name": "coexistence__read_probe", "tool_input": {},
                })

    def observe_terminal(self, events):
        if not self.rejected or not self.pending or self.pending[0] != "mcp":
            return False
        call_id = self.pending[1]
        rejected = [event for event in events if event.get("type") == "chat.tool"
                    and event.get("call_id") == call_id and event.get("kind") == "complete"
                    and event.get("status") == "failed"
                    and event.get("payload", {}).get("error")
                    == 'User rejected the execution for tool `use_tool`']
        if len(rejected) != 1:
            return False
        self.completed["mcp"] = call_id
        return True
