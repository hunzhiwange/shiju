"""Probe GLM 1.17.1's model and ACP slash-command discovery paths."""
from contextlib import contextmanager
import hashlib
import json
from pathlib import Path
import platform
import secrets
import shutil
import subprocess
import sys
import tempfile

from verify_acp_approval import environment
from verify_acp_integration import Process, ProviderHandler, require
from verify_acp_model_source import ROOT

AGENT = "glm-acp-agent"
VERSION = "1.17.1"
MODEL = "deepseek-v4.1-flash-1017"
MODEL_OUTPUT_LIMIT = 32_768
ADAPTER = "openai"
API_PATH = "/v1"
MODEL_ENDPOINT = API_PATH + "/chat/completions"
TRANSPORT = "http" if platform.system() == "Linux" else "stdio"
NATIVE_DIR = ".claude/skills"
CATALOG = "Available skills. Use the native read_file tool on a listed path to load instructions when needed.\n"


@contextmanager
def mcp_server(_workspace, attachment, audit):
    from acp_mount_integration import McpHandler
    from verify_acp_skill_coexistence import mcp_server as stdio_fixture, model_server

    if TRANSPORT == "stdio":
        with stdio_fixture("stdio", attachment, audit) as server:
            yield server
        return

    class HttpFixture(McpHandler):
        def do_POST(self):
            if self.path != "/mcp" or self.headers.get("Authorization") != "Bearer " + self.server.key:
                self.reply(401)
                return
            super().do_POST()

    key = secrets.token_hex(24)
    with model_server(HttpFixture, key) as server:
        server.attachment = attachment
        server.audit = audit
        yield {"name": "coexistence", "type": "http",
               "url": f"http://127.0.0.1:{server.server_port}/mcp",
               "headers": [{"name": "Authorization", "value": "Bearer " + key}]}


def launch(env):
    require((platform.system(), platform.machine()) in {
                ("Darwin", "arm64"), ("Linux", "x86_64"), ("Linux", "aarch64"),
            },
            "unreviewed GLM coexistence platform")
    installer = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime"
    contracts = installer / "contracts.json"
    contract = json.loads(contracts.read_text())[AGENT]
    require(contract["version"] == VERSION, "unexpected GLM package version")
    config = Path(env["VIBEWINDOW_CONFIG_DIR"])
    assets = config / "glm-install-assets"
    assets.mkdir(mode=0o700)
    shutil.copyfile(contracts, assets / "contracts.json")
    sources = ROOT / "patches/acp-model-source" / AGENT
    shutil.copyfile(sources / f"{AGENT}-{VERSION}-skills.patch", assets / f"{AGENT}.patch")
    hashes = dict(contract["files"])
    args = []
    revision = f"{AGENT}-{VERSION}-model-source-v3"
    if platform.system() == "Linux":
        overlay = sources / "linux-host-mcp.json"
        hashes.update(json.loads(overlay.read_text())["files"])
        shutil.copyfile(overlay, assets / "glm-linux-contract.json")
        shutil.copyfile(sources / f"{AGENT}-{VERSION}-linux-host-mcp.patch",
                        assets / "glm-linux-session-model.patch")
        args = ["", "--glm-linux-v4"]
        revision = f"{AGENT}-{VERSION}-model-source-v5"
    runtime = config / "acp-runtime" / revision
    runtime.parent.mkdir(mode=0o700)
    installed = subprocess.run([sys.executable, str(installer / "install.py"),
                                str(assets), str(runtime), AGENT, *args],
                               env=env, capture_output=True, timeout=100)
    require(installed.returncode == 0, "reviewed GLM private installation failed")
    package = runtime / "node_modules" / AGENT
    for name, digest in hashes.items():
        require(hashlib.sha256((package / name).read_bytes()).hexdigest() == digest,
                f"reviewed GLM checksum mismatch: {name}")
    return {"command": str(package / contract["bin"]), "args": []}, {
        "version": VERSION, "revision": revision, "entry": contract["bin"],
        "integrity": contract["integrity"], "sha256": hashes,
        "model_endpoint": MODEL_ENDPOINT,
    }


class Probe:
    def __init__(self, fixtures, mounted, rejected, *, slash=False):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.slash = slash
        self.mcp_tool = ("mcp__coexistence__read_probe"
                         if not slash and platform.system() == "Linux" else "read_probe")
        self.requests = 0
        self.pending = None
        self.completed = {}
        self.failure = None
        self.paths = {}

    def response(self, body):
        tools = {row["function"]["name"]: row["function"] for row in body.get("tools", [])}
        if not tools:
            return {"content": "GLM fixture session"}, "stop"
        require(body["model"] == MODEL and body.get("stream") is True,
                "unexpected GLM Chat Completions request")
        self.requests += 1
        native = self.fixtures["skills"]["native"]
        messages = body["messages"]
        wire = json.dumps(body)
        if self.requests == 1:
            for kind, skill in self.fixtures["skills"].items():
                if not (self.slash and kind == "native"):
                    require(skill["body"] not in wire, "GLM included a skill body before activation")
                require(skill["attachment"] not in wire, "GLM included a skill attachment before reading")
            require(self.fixtures["mcp"] not in wire, "GLM included the MCP result before invocation")
            require("read_file" in tools, "GLM omitted its native file reader")
            if not self.slash:
                if platform.system() == "Linux":
                    require(not {"image_analysis", "web_search", "web_reader"} & tools.keys(),
                            "GLM managed session exposed an unapproved vendor MCP tool")
                catalogs = [row["content"].split(CATALOG, 1)[1] for row in messages
                            if row["role"] == "system" and CATALOG in row["content"]]
                require(len(catalogs) == 1, "missing or ambiguous GLM model skill catalog")
                entries = json.loads(catalogs[0])
                expected = ("native", "selected") if self.mounted else ("native",)
                skills = self.fixtures["skills"]
                paths = {row["name"]: Path(row["path"]) for row in entries}
                require(len(entries) == len(paths) == len(expected)
                        and paths.keys() == {skills[kind]["name"] for kind in expected},
                        "GLM catalog differs from the trusted selection")
                for kind in expected:
                    path = paths[skills[kind]["name"]]
                    require(path == (skills[kind]["path"] / "SKILL.md").resolve(),
                            f"unexpected GLM {kind} skill path")
                    self.paths[kind] = path
                require((self.mcp_tool in tools) == self.mounted, "unexpected GLM session MCP directory")
                return self.read(tools, "native")
            source = native["path"] / "SKILL.md"
            require(any(row["role"] == "user" and native["body"] in row["content"]
                        and f'source="{source}"' in row["content"] for row in messages),
                    "GLM did not expand the advertised native slash command")
            require(("read_probe" in tools) == (self.mounted and not self.rejected),
                    "unexpected GLM stdio MCP directory")
            return self.call(tools, "native_read", "read_file",
                             {"path": str(source.parent / "references/value.txt")})

        step, call_id = self.pending
        results = [row["content"] for row in messages
                   if row.get("role") == "tool" and row.get("tool_call_id") == call_id]
        if step == "mcp":
            value = self.fixtures["mcp"]
        else:
            skill = self.fixtures["skills"][step.removesuffix("_read")]
            value = skill["attachment" if step.endswith("_read") else "body"]
        require(len(results) == 1, f"missing GLM {step} continuation")
        if step == "mcp" and self.rejected:
            reason = "unknown tool" if self.slash else "rejected by user"
            require(value not in results[0] and reason in results[0],
                    "denied GLM MCP call did not fail locally")
        else:
            require(value in results[0], f"missing GLM {step} result")
        self.completed[step] = call_id
        if step in ("native", "selected"):
            return self.read(tools, step + "_read")
        if self.mounted and step == "native_read" and not self.slash:
            return self.read(tools, "selected")
        if self.mounted and step in ("native_read", "selected_read"):
            # The direct policy control intentionally tries the excluded tool.
            return self.call(tools, "mcp", self.mcp_tool, {})
        return {"content": "GLM skill coexistence fixture finished"}, "stop"

    def read(self, tools, step):
        path = self.paths[step.removesuffix("_read")]
        if step.endswith("_read"):
            path = path.parent / "references/value.txt"
        return self.call(tools, step, "read_file", {"path": str(path)})

    def call(self, tools, step, name, arguments):
        if not (step == "mcp" and self.slash and self.rejected):
            schema = tools[name]["parameters"]
            require(set(schema.get("required", ())) <= arguments.keys() <= schema["properties"].keys(),
                    f"unexpected GLM {name} schema")
        call_id = "call_" + secrets.token_hex(12)
        self.pending = step, call_id
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def approves(self, metadata):
        return (self.pending is not None and self.pending[0] == "mcp"
                and metadata.get("title") == self.mcp_tool and metadata.get("arguments") == {})


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.path == MODEL_ENDPOINT, "unexpected GLM model endpoint")
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated GLM model request")
            delta, finish = probe.response(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
        except (AssertionError, KeyError, TypeError, ValueError) as error:
            probe.failure = str(error) if isinstance(error, AssertionError) else "invalid GLM model request"
            self.send_error(422, "GLM probe failed")
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.chunk(MODEL, dict(delta, role="assistant"), None)
        self.chunk(MODEL, {}, finish)
        self.wfile.write(b"data: [DONE]\n\n")


def direct_control():
    """Observe the provider's client-only menu and the managed stdio dispatch."""
    from verify_acp_skill_coexistence import make_skill, model_server

    with tempfile.TemporaryDirectory(prefix="shiju-glm-slash-") as directory:
        root = Path(directory).resolve()
        env = environment(root)
        spec, artifact = launch(env)
        workspace = root / "workspace"
        native = make_skill(workspace / NATIVE_DIR, "native")
        attachment = root / "mcp.txt"
        attachment.write_text(secrets.token_hex(24))
        fixtures = {"skills": {"native": native}, "mcp": attachment.read_text()}
        audit = root / "audit.txt"
        key = secrets.token_hex(24)
        with model_server(Handler, key) as model, mcp_server(workspace, attachment, audit) as mcp:
            env.update(ACP_GLM_MODEL=MODEL, ACP_GLM_AVAILABLE_MODELS=MODEL,
                       ACP_GLM_BASE_URL=f"http://127.0.0.1:{model.server_port}{API_PATH}",
                       Z_AI_API_KEY=key, XDG_CONFIG_HOME=env["VIBEWINDOW_CONFIG_DIR"],
                       ACP_GLM_SESSION_DIR=str(root / "sessions"))
            process = Process(["node", spec["command"]], workspace, env, root / "agent.log")
            rows = []
            try:
                info, _ = process.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
                require(info["agentInfo"]["version"] == VERSION, "unexpected GLM ACP version")
                denied = {**mcp, "_meta": {"shiju.glm.toolPolicy": {"allow": [], "deny": ["read_probe"]}}}
                overlap = {**mcp, "_meta": {"shiju.glm.toolPolicy": {
                    "allow": ["read_probe"], "deny": ["read_probe"],
                }}}
                for phase, servers in (("baseline", []), ("mounted", [mcp]),
                                       ("removed", []), ("policy_denied", [denied]),
                                       ("policy_overlap", [overlap])):
                    session, updates = process.call("session/new", {"cwd": str(workspace), "mcpServers": servers})
                    model.probe = probe = Probe(fixtures, bool(servers), phase.startswith("policy_"), slash=True)
                    before = audit.read_text().splitlines() if audit.exists() else []
                    result, events = process.call("session/prompt", {"sessionId": session["sessionId"],
                        "prompt": [{"type": "text", "text": "/" + native["name"]}]})
                    require(not probe.failure, probe.failure)
                    require(result["stopReason"] == "end_turn", "GLM slash prompt did not finish")
                    require(any(update.get("sessionUpdate") == "available_commands_update"
                                and any(row["name"] == native["name"] for row in update["availableCommands"])
                                for event in updates + events
                                if (update := event.get("params", {}).get("update"))),
                            "GLM ACP omitted the native slash menu entry")
                    expected_calls = {"native_read", "mcp"} if servers else {"native_read"}
                    require(set(probe.completed) == expected_calls
                            and probe.requests == len(expected_calls) + 1,
                            "incomplete GLM slash control continuation")
                    approvals = sum(event.get("method") == "session/request_permission"
                                    for event in updates + events)
                    require(approvals == 0, "GLM sent an unexpected permission request")
                    after = audit.read_text().splitlines() if audit.exists() else []
                    require(after == before + ([TRANSPORT] if phase == "mounted" else []),
                            "incorrect direct GLM MCP audit count")
                    rows.append({"phase": phase, "session": session["sessionId"], "stop": result["stopReason"],
                                 "model_requests": probe.requests, "native_slash_discovered": True,
                                 "calls": probe.completed, "approval_requests": approvals,
                                 "mcp_calls": len(after) - len(before)})
            finally:
                process.close()
            require(process.child.returncode == 0, "GLM ACP process did not exit cleanly")
            require(len({row["session"] for row in rows}) == len(rows), "GLM reused an ACP session")
            return {"artifact": artifact, "model": MODEL, "turns": rows}


if __name__ == "__main__":
    print(json.dumps(direct_control(), indent=2))
