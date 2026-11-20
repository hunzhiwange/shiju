"""Claude 0.26.0 native Messages and project Skill probe."""
import hashlib
import json
import platform
import secrets
from http.server import BaseHTTPRequestHandler
from pathlib import Path

from acp_skill_mount_claude import messages_wire
from verify_acp_integration import require
from verify_acp_model_source import ROOT, claude_sdk_identity, locate

from . import correlated_result

MODEL = "claude-sonnet-4-6"
ADAPTER = "anthropic"
API_PATH = "/v1"
TRANSPORT = "stdio"
MCP_TOOL = "mcp__coexistence__read_probe"
NATIVE_DIR = ".claude/skills"


def launch(env):
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"), "unreviewed Claude platform")
    manifest = json.loads((ROOT / "docs/shiju/acp-model-source/cli-versions.json").read_text())
    entry = next(item for item in manifest["entries"] if item["key"] == "claude")
    spec = locate(entry, env)
    evidence = json.loads((ROOT / "docs/vibewindow/acp-skill-mounts/evidence/claude.json").read_text())
    package = Path(spec["command"]).resolve().parents[1]
    hashes = {}
    for label, digest in evidence["identity"]["sha256"].items():
        relative = label.split("/claude-agent-acp/", 1)[1]
        actual = hashlib.sha256((package / relative).read_bytes()).hexdigest()
        require(actual == digest, "Claude artifact digest differs from reviewed release")
        hashes[relative] = actual
    identity = claude_sdk_identity(spec["command"], entry, env)
    require(identity == {"adapter": "0.26.0", "sdk": "0.2.96", "engine": "2.1.96"},
            "Claude version differs from coexistence pin")
    return spec, dict(identity, sha256=hashes)


class Probe:
    """Map observed tool declarations and Messages results to the next read-only action."""
    def __init__(self, fixtures, mounted, rejected):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.requests = 0
        self.calls = {}
        self.completed = {}
        self.failure = None
        self.mcp_pending = None

    def response(self, body):
        # The relay's upstream model ID must match the configured fixture model.
        require(body["model"] == MODEL, "unexpected upstream model")
        tools = {tool["name"]: tool for tool in body.get("tools", [])}
        messages = body["messages"]
        self.requests += 1
        if self.requests == 1:
            catalog = json.dumps([body.get("system"), messages, list(tools.values())])
            for kind, skill in self.fixtures["skills"].items():
                discovered = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in catalog) == discovered, f"unexpected {kind} skill catalog")
                require(skill["body"] not in catalog and skill["attachment"] not in catalog,
                        "skill content appeared before activation")
            require(self.fixtures["mcp"] not in catalog, "MCP value appeared before invocation")
            require((MCP_TOOL in tools) == self.mounted, "unexpected MCP tool catalog")

        for kind in ("native", "selected"):
            if kind == "selected" and not self.mounted:
                break
            skill = self.fixtures["skills"][kind]
            if kind not in self.calls:
                name = skill["name"] if kind == "native" else "shiju-session-skills:" + skill["name"]
                return self.call(tools, kind, "Skill", {"skill": name})
            read = kind + "_read"
            if read not in self.calls:
                expansion = self.activation(messages, kind, skill)
                # Use the directory emitted by the native expansion. The selected
                # fixture is read from the plugin's session projection.
                lines = expansion.splitlines()
                prefix = "Base directory for this skill: "
                base = next((Path(line[len(prefix):]) for line in lines if line.startswith(prefix)), None)
                require(base is not None and base.is_absolute(), "skill expansion lacks a base directory")
                path = base / "references/value.txt"
                require(base.name == skill["name"] and base.resolve() == skill["path"].resolve(),
                        "unexpected skill read target")
                if kind == "selected":
                    require(base.parent.name == "skills" and
                            base.parent.parent.name.startswith("shiju-session-skills-"),
                            "selected skill bypassed the session view")
                return self.call(tools, read, "Read", {"file_path": str(path)})
            correlated_result(self.results(messages), self.calls[read], skill["attachment"])
            self.completed[read] = self.calls[read]
        if self.mounted:
            if "mcp" not in self.calls:
                response = self.call(tools, "mcp", MCP_TOOL, {})
                self.mcp_pending = self.calls["mcp"]
                return response
            correlated_result(self.results(messages), self.calls["mcp"],
                              "User refused permission to run tool" if self.rejected else self.fixtures["mcp"],
                              rejected=self.rejected)
            self.completed["mcp"] = self.calls["mcp"]
            self.mcp_pending = None
        return {"content": "COEXISTENCE_DONE"}

    @staticmethod
    def results(messages):
        return [block for message in messages if message["role"] == "user"
                and isinstance(message["content"], list) for block in message["content"]
                if block["type"] == "tool_result"]

    def activation(self, messages, kind, skill):
        call_id = self.calls[kind]
        result = correlated_result(self.results(messages), call_id, "Launching skill:")
        # Claude serializes the Skill body as a user expansion after the launch
        # result. Bind it to that result before considering another tool block.
        after_launch = False
        for message in messages:
            content = message["content"]
            blocks = content if isinstance(content, list) else [{"type": "text", "text": content}]
            for block in blocks:
                if block["type"] == "tool_result":
                    after_launch = block == result
                elif after_launch and block["type"] == "text" and message["role"] == "user":
                    if skill["body"] in block["text"]:
                        self.completed[kind] = call_id
                        return block["text"]
                elif block["type"] == "tool_use":
                    after_launch = False
        raise AssertionError(f"{kind} skill body missing from native continuation")

    def call(self, tools, kind, name, arguments):
        require(name in tools, f"{kind} tool missing from native catalog")
        schema = tools[name]["input_schema"]
        require(set(arguments) <= set(schema.get("properties", {}))
                and set(schema.get("required", ())) <= set(arguments), f"{kind} tool schema changed")
        call_id = "toolu_" + secrets.token_hex(12)
        self.calls[kind] = call_id
        return {"tool_calls": [{"id": call_id, "function": {
            "name": name, "arguments": json.dumps(arguments)}}]}

    def approves(self, metadata):
        return (self.mcp_pending is not None and metadata.get("title") == MCP_TOOL
                and metadata.get("arguments") in ({}, None))


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_POST(self):
        fixture = self.server.probe
        code = 200
        content_type = "application/json"
        data = b'{"input_tokens": 1}'
        try:
            require(self.headers.get("x-api-key") == self.server.key
                    or self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated model request")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid model request size")
            body = json.loads(self.rfile.read(size))
            route = self.path.split("?", 1)[0]
            if route == "/v1/messages":
                response, wire = messages_wire(fixture.response(body), body["model"])
                if body.get("stream"):
                    data, content_type = wire, "text/event-stream"
                else:
                    data = json.dumps(response).encode()
            else:
                require(route == "/v1/messages/count_tokens", "unexpected model protocol")
        except AssertionError as error:
            fixture.failure = fixture.failure or str(error)
            code, data = 422, b'{"error":"probe rejected model request"}'
        except (KeyError, TypeError, ValueError):
            fixture.failure = fixture.failure or "invalid Messages envelope"
            code, data = 422, b'{"error":"invalid Messages envelope"}'
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
