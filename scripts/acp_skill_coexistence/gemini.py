"""Gemini 0.66.0 native GenerateContent, skill activation and stdio MCP probe."""

import hashlib
from http.server import BaseHTTPRequestHandler
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
from urllib.parse import urlsplit

from acp_skill_mount_gemini import generate_wire
from verify_acp_integration import require
from verify_acp_model_source import ROOT

MODEL = "gemini-2.5-flash"
ADAPTER = "google"
API_PATH = "/v1beta"
TRANSPORT = "stdio"
NATIVE_DIR = ".gemini/skills"
MCP_TOOL = "mcp_coexistence_read_probe"
REVISION = "gemini-0.66.0-model-source-v3"


def launch(env):
    system, machine = platform.system(), platform.machine()
    linux = system == "Linux" and machine in ("x86_64", "aarch64")
    require(linux or (system, machine) == ("Darwin", "arm64"),
            "unreviewed Gemini platform")
    if linux:
        require(platform.libc_ver()[0] == "glibc", "Gemini Linux fixture requires glibc")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json")
                          .read_text())["gemini"]
    require(contract["version"] == "0.66.0", "Gemini version differs from coexistence pin")
    require(shutil.which("node", path=env["PATH"]), "Node is unavailable")
    base = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    runtime = Path(os.environ.get("GEMINI_COEXISTENCE_RUNTIME", base / "acp-runtime" / REVISION))
    require((runtime / "node_modules" / contract["package"]).is_dir(),
            "reviewed Gemini package is unavailable")
    config = Path(env["VIBEWINDOW_CONFIG_DIR"])
    private = config / "acp-runtime" / REVISION
    shutil.copytree(runtime, private, symlinks=True)
    package = private / "node_modules" / contract["package"]
    metadata = json.loads((package / "package.json").read_text())
    require((metadata["name"], metadata["version"]) == (contract["package"], contract["version"]),
            "Gemini package identity differs from contract")
    if linux:
        lock = json.loads((private / "package-lock.json").read_text())["packages"]
        require(lock["node_modules/" + contract["package"]]["integrity"] == contract["integrity"],
                "Gemini package integrity differs from contract")
        require(not any((private / "node_modules" / name).exists()
                        for name in metadata.get("optionalDependencies", {})),
                "Gemini Linux fixture requires the reviewed JavaScript-only installation")
    for name, digest in contract["files"].items():
        require(hashlib.sha256((package / name).read_bytes()).hexdigest() == digest,
                "Gemini artifact digest differs from contract")
    entry = private / "node_modules/.bin/gemini"
    require(entry.resolve() == (package / contract["bin"]).resolve(),
            "Gemini launcher differs from the reviewed entry")
    # Enable native folder trust and grant it only to this private workspace.
    root = config.parent
    trust = root / "gemini-trusted-folders.json"
    trust.write_text(json.dumps({str(root / "workspace"): "TRUST_FOLDER"}))
    defaults = root / "gemini-system-defaults.json"
    defaults.write_text(json.dumps({"security": {"folderTrust": {"enabled": True}}}))
    env.update(GEMINI_CLI_TRUSTED_FOLDERS_PATH=str(trust),
               GEMINI_CLI_SYSTEM_DEFAULTS_PATH=str(defaults))
    return {"command": str(entry), "args": ["--experimental-acp"], "env": {}}, {
        "version": contract["version"], "sha256": contract["files"], "revision": REVISION,
        "entry": contract["bin"], "integrity": contract["integrity"],
        "model_endpoint": f"{API_PATH}/models/{MODEL}:streamGenerateContent"}


class Probe:
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.calls = {}
        self.completed = {}
        self.failure = None
        self.pending = None

    def response(self, body):
        require(self.failure is None, self.failure)
        self.requests += 1
        tools = {tool["name"]: tool for group in body.get("tools", [])
                 for tool in group.get("functionDeclarations", [])}
        parts = [part for content in body["contents"] for part in content["parts"]]
        results = [part["functionResponse"] for part in parts if "functionResponse" in part]
        catalog = "\n".join(re.findall(r"<available_skills>(.*?)</available_skills>",
                                       json.dumps([body.get("systemInstruction"), body["contents"]]),
                                       re.S))
        if self.requests == 1:
            request_text = json.dumps(body)
            for skill in self.fixtures["skills"].values():
                require(skill["body"] not in request_text and skill["attachment"] not in request_text,
                        "skill content appeared before activation")
            require(self.fixtures["mcp"] not in request_text, "MCP value appeared before invocation")
        require(self.mounted or MCP_TOOL not in tools, "unselected MCP tool was advertised")
        for kind, skill in self.fixtures["skills"].items():
            expected = kind == "native" or (kind == "selected" and self.mounted)
            require((f'<name>{skill["name"]}</name>' in catalog) == expected,
                    f"unexpected {kind} skill catalog")

        for kind in ("native", "selected") if self.mounted else ("native",):
            skill = self.fixtures["skills"][kind]
            if kind not in self.calls:
                return self.call(tools, kind, "activate_skill", {"name": skill["name"]})
            expansion = self.result(results, kind)
            require(skill["body"] in expansion and "references" in expansion,
                    f"{kind} activation did not return instructions and resources")
            read = kind + "_read"
            if read not in self.calls:
                locations = re.findall(r"<location>(.*?)</location>", catalog)
                paths = [Path(path) for path in locations if Path(path).parent.name == skill["name"]]
                require(len(set(paths)) == 1, f"missing or ambiguous {kind} skill location")
                marker = paths[0]
                source = skill["path"] / "SKILL.md"
                require(marker.is_absolute() and marker.resolve() == source.resolve()
                        and (marker == source) == (kind == "native"),
                        f"{kind} skill was loaded from the wrong root")
                require(str(marker.parent) + "/" in expansion,
                        f"{kind} activation did not return the advertised directory")
                return self.call(tools, read, "read_file",
                                 {"file_path": str(marker.parent / "references/value.txt")})
            require(skill["attachment"] in self.result(results, read), f"{kind} attachment missing")

        if self.mounted:
            if "mcp" not in self.calls:
                return self.call(tools, "mcp", MCP_TOOL, {})
            outcome = self.result(results, "mcp")
            if self.rejected:
                require(re.search(r"reject|denied|cancel", outcome, re.I)
                        and self.fixtures["mcp"] not in outcome, "MCP rejection was not observed")
            else:
                require(self.fixtures["mcp"] in outcome, "MCP result lacks the fixture value")
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}

    def result(self, results, kind):
        call_id, name = self.calls[kind]
        matches = [item["response"] for item in results
                   if item.get("id") == call_id and item.get("name") == name]
        require(len(matches) == 1, f"missing or ambiguous {kind} result")
        require(("error" in matches[0]) == (kind == "mcp" and self.rejected),
                f"unexpected {kind} result status")
        self.completed[kind] = call_id
        return json.dumps(matches[0])

    def call(self, tools, kind, name, args):
        require(name in tools, f"{name} missing from native catalog")
        schema = tools[name].get("parameters", tools[name].get("parametersJsonSchema", {}))
        require(set(schema.get("required", [])) <= set(args) <= set(schema.get("properties", {})),
                f"{name} tool schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.calls[kind] = (call_id, name)
        self.pending = kind
        return {"tool_calls": [{"id": call_id, "function": {"name": name,
                                                          "arguments": json.dumps(args)}}]}

    def approves_skill(self, metadata):
        kind = self.pending
        if kind not in ("native", "selected") or kind in self.completed:
            return None
        name = self.fixtures["skills"][kind]["name"]
        if metadata.get("title") == f"Activate Skill: {name}" and metadata.get("arguments") == {"name": name}:
            return kind
        return None

    def approves(self, metadata):
        return (self.pending == "mcp" and "mcp" not in self.completed
                and metadata.get("title") == "read_probe (coexistence MCP Server)"
                and metadata.get("arguments") in (None, {}))


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_POST(self):
        probe = self.server.probe
        code, content_type = 200, "application/json"
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key
                    or self.headers.get("x-goog-api-key") == self.server.key,
                    "unauthenticated model request")
            prefix = f"{API_PATH}/models/{MODEL}:"
            route = urlsplit(self.path).path
            require(route.startswith(prefix), "unexpected Gemini model route")
            action = route[len(prefix):]
            require(action in ("generateContent", "streamGenerateContent", "countTokens"),
                    "unexpected Gemini model action")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid model request size")
            body = json.loads(self.rfile.read(size))
            if action == "countTokens":
                data = b'{"totalTokens":1}'
            else:
                message = (probe.response(body) if action == "streamGenerateContent"
                           else {"content": "Ready"})
                value, stream = generate_wire(message)
                if action == "streamGenerateContent":
                    data, content_type = stream, "text/event-stream"
                else:
                    data = json.dumps(value).encode()
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid GenerateContent envelope")
            code, data = 422, b'{"error":"probe rejected model request"}'
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
