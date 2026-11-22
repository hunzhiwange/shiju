"""Verify iFlow 0.5.19's native Skill, file reader and stdio MCP tools."""

import hashlib
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
import subprocess

from verify_acp_integration import ProviderHandler, require
from verify_acp_model_source import ROOT

MODEL = "deepseek-v4.1-flash-1017"
ADAPTER = "openai"
API_PATH = "/v1"
TRANSPORT = "stdio"
NATIVE_DIR = ".iflow/skills"
MCP_TOOL = "read_probe"


def launch(env):
    if platform.system() == "Linux":
        return launch_linux(env)
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unreviewed iFlow platform")
    contract = json.loads((ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json")
                          .read_text())["iflow"]
    require(contract["version"] == "0.5.19", "unexpected iFlow contract version")
    original = Path(os.environ.get("VIBEWINDOW_CONFIG_DIR", Path.home() / ".local/share/shiju"))
    for revision in ("iflow-0.5.19-model-source-v2", "iflow-0.5.19-model-source-v1"):
        source = original / "acp-runtime" / revision / "node_modules"
        package = source / "@iflow-ai/iflow-cli"
        if all((package / name).is_file()
               and hashlib.sha256((package / name).read_bytes()).hexdigest() == digest
               for name, digest in contract["files"].items()):
            break
    else:
        raise AssertionError("reviewed iFlow runtime is unavailable")
    target = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision / "node_modules"
    shutil.copytree(source, target, symlinks=True)
    command = target / ".bin/iflow"
    require(command.resolve() == target / "@iflow-ai/iflow-cli" / contract["bin"],
            "unexpected iFlow entrypoint")
    version = subprocess.run([str(command), "--version"], cwd=target, env=env,
                             capture_output=True, timeout=15, check=False)
    require(version.returncode == 0 and version.stdout.strip() == b"0.5.19",
            "iFlow entry did not report the pinned version")
    return {"command": str(command), "args": ["--experimental-acp"]}, {
        "version": contract["version"], "revision": revision, "sha256": contract["files"],
    }


def launch_linux(env):
    manifest = json.loads((ROOT / "patches/acp-model-source/iflow/linux-mcp-artifacts.json").read_text())
    target = manifest["targets"].get(platform.machine())
    require(target is not None, "unreviewed iFlow Linux architecture")
    source = os.environ.get("IFLOW_COEXISTENCE_RUNTIME")
    require(source, "set IFLOW_COEXISTENCE_RUNTIME to the reviewed Linux installation")

    revision = manifest["revision"]
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / revision
    shutil.copytree(Path(source).resolve(strict=True), runtime, symlinks=True)
    modules = runtime / "node_modules"
    package = modules / manifest["package"]
    entry = package / manifest["bin"]
    command = modules / ".bin/iflow"

    def digest(path):
        require(path.resolve(strict=True) == path, "redirected iFlow artifact")
        return hashlib.sha256(path.read_bytes()).hexdigest()

    lock_path = runtime / "package-lock.json"
    require(digest(lock_path) == manifest["lock_sha256"], "unreviewed iFlow dependency lock")
    lock = json.loads(lock_path.read_text())["packages"]
    integrity = lock["node_modules/" + manifest["package"]]["integrity"]
    require(integrity == manifest["integrity"], "iFlow npm archive integrity mismatch")
    observed = {name: digest(package / name) for name in manifest["patched_files"]}
    require(observed == manifest["patched_files"], "iFlow managed bundle mismatch")

    ripgrep = package / target["ripgrep"]
    require(os.access(ripgrep, os.X_OK), "iFlow bundled ripgrep is not executable")
    ripgrep_digest = digest(ripgrep)
    pty_digest = digest(modules / target["pty_package"] / "pty.node")
    require(ripgrep_digest == target["ripgrep_sha256"] and pty_digest == target["pty_sha256"],
            "iFlow Linux native dependency mismatch")
    require(command.resolve(strict=True) == entry, "unexpected iFlow entrypoint")
    version = subprocess.run([str(command), "--version"], cwd=runtime, env=env,
                             capture_output=True, timeout=15, check=False)
    require(version.returncode == 0 and version.stdout.strip().decode() == manifest["version"],
            "iFlow entry did not report the pinned version")
    args = ["--experimental-acp"]
    return {"command": str(command), "args": args}, {
        "version": version.stdout.strip().decode(), "revision": revision,
        "entry": manifest["bin"], "args": args, "sha256": observed, "integrity": integrity,
        "ripgrep_sha256": ripgrep_digest, "pty_sha256": pty_digest,
    }


def message_text(content):
    if isinstance(content, list):
        return "".join(part["text"] for part in content if part.get("type") == "text")
    return content if isinstance(content, str) else json.dumps(content)


class Probe:
    def __init__(self, fixtures, mounted, rejected, *, mcp=None, current="", previous=""):
        self.fixtures = fixtures
        self.mounted = mounted
        self.rejected = rejected
        self.mcp = mounted if mcp is None else mcp
        self.current = current
        self.previous = previous
        self.requests = 0
        self.calls = {}
        self.completed = {}
        self.failure = None
        self.pending = None
        self.pending_read = None
        self.native_read_decision = None
        self.native_read_denied = False

    def response(self, body):
        require(body["model"] == MODEL, "unexpected iFlow model")
        tools = {item["function"]["name"]: item["function"] for item in body.get("tools", [])}
        self.requests += 1
        messages = body["messages"]
        if self.native_read_decision:
            require(self.fixtures["skills"]["native"]["attachment"] not in json.dumps(messages),
                    "blocked iFlow read exposed its attachment to the model")
            require(self.native_read_decision != "cancel" or "native_read" not in self.calls,
                    "cancelled iFlow read reached a model continuation")
        if self.requests == 1:
            catalog = tools.get("Skill", {}).get("description", "")
            sections = re.findall(r"<available_skills>(.*?)</available_skills>", catalog, re.S)
            names = set(re.findall(r"<name>\s*([^<\s]+)\s*</name>", "".join(sections)))
            current_messages = messages
            if self.current:
                current = next((index for index, row in enumerate(messages)
                                if row["role"] == "user" and self.current in message_text(row["content"])), None)
                require(current is not None, "iFlow did not receive the current prompt")
                if self.previous:
                    require(any(row["role"] == "user" and self.previous in message_text(row["content"])
                                for row in messages[:current]), "iFlow did not restore the previous prompt")
                # Already-approved historical reads may remain in the transcript.
                current_messages = [row for row in messages[:current] if row["role"] == "system"]
                current_messages += messages[current:]
            payload = json.dumps({**body, "messages": current_messages})
            for kind, skill in self.fixtures["skills"].items():
                visible = kind == "native" or (kind == "selected" and self.mounted)
                require((skill["name"] in names) == visible, f"unexpected iFlow {kind} catalog")
                require(skill["body"] not in payload and skill["attachment"] not in payload,
                        "iFlow skill content leaked before activation")
            require(self.fixtures["mcp"] not in payload, "MCP value leaked before invocation")
            require((MCP_TOOL in tools) == self.mcp, "unexpected iFlow MCP catalog")

        for kind in ("native", "selected") if self.mounted else ("native",):
            skill = self.fixtures["skills"][kind]
            if kind not in self.calls:
                return self.call(tools, kind, "Skill", {"skill": skill["name"]})
            result, index = self.result(messages, kind)
            if result.strip() == '{"error":"iFlow tool execution unavailable in this prompt"}':
                raise AssertionError("iFlow blocked the skill before requesting approval")
            require("Launching skill: " + skill["name"] in result, "incorrect iFlow skill activation")
            self.completed[kind] = self.calls[kind]
            read = kind + "_read"
            if read not in self.calls:
                later_users = (message_text(row["content"]) for row in messages[index + 1:]
                               if row["role"] == "user")
                expanded = [content for content in later_users if skill["body"] in content]
                require(len(expanded) == 1, "iFlow skill body did not reach model")
                match = re.search(r"Base directory for this skill:\s*([^\n]+)", expanded[0])
                require(match is not None, "iFlow skill base missing")
                base = Path(match[1].strip())
                require(base.is_absolute() and base.name == skill["name"] and
                        base.resolve() == skill["path"].resolve(), "wrong iFlow skill path")
                if kind == "native":
                    require(base == skill["path"], "iFlow bypassed its native skill directory")
                else:
                    require(base.parent.parent.name.startswith("shiju-session-skills-"),
                            "iFlow bypassed the session skill view")
                self.pending_read = {"absolute_path": str(base / "references/value.txt")}
                return self.call(tools, read, "read_file", self.pending_read)
            result, _ = self.result(messages, read)
            if read == "native_read" and self.native_read_decision == "reject":
                require("Tool execution rejected by user" in result, "iFlow native read refusal missing")
                self.native_read_denied = True
                self.completed[read] = self.calls[read]
                continue
            require(skill["attachment"] in result,
                    f"iFlow {kind} attachment was not read")
            self.completed[read] = self.calls[read]

        if self.mcp:
            if "mcp" not in self.calls:
                return self.call(tools, "mcp", MCP_TOOL, {})
            result, _ = self.result(messages, "mcp")
            if self.rejected:
                require(re.search(r"reject|denied|cancel", result, re.I)
                        and self.fixtures["mcp"] not in result, "iFlow MCP refusal missing")
            else:
                require(self.fixtures["mcp"] in result, "iFlow MCP value was not read")
            self.completed["mcp"] = self.calls["mcp"]
        self.pending = None
        return {"content": "COEXISTENCE_DONE"}, "stop"

    def call(self, tools, kind, name, arguments):
        require(name in tools, f"iFlow {kind} tool missing")
        schema = tools[name]["parameters"]
        require(set(schema.get("required", [])) <= set(arguments) <= set(schema.get("properties", {})),
                f"iFlow {kind} tool schema changed")
        call_id = "call_" + secrets.token_hex(12)
        self.calls[kind] = call_id
        self.pending = kind
        return {"tool_calls": [{"index": 0, "id": call_id, "type": "function",
                "function": {"name": name, "arguments": json.dumps(arguments)}}]}, "tool_calls"

    def result(self, messages, kind):
        call_id = self.calls[kind]
        results = [(row["content"], index) for index, row in enumerate(messages)
                   if row["role"] == "tool" and row.get("tool_call_id") == call_id]
        require(len(results) == 1, f"iFlow {kind} result missing or ambiguous")
        content, index = results[0]
        return message_text(content), index

    def approves(self, metadata):
        arguments = metadata.get("arguments")
        title = "mcp__coexistence__" + MCP_TOOL if platform.system() == "Linux" else MCP_TOOL
        return (self.pending == "mcp" and metadata.get("title") == title
                and (arguments == {} or (platform.system() != "Linux" and arguments is None)))

    def expected_skill_approvals(self):
        sources = ("native", "selected") if self.mounted else ("native",)
        if platform.system() == "Linux":
            return [step for source in sources for step in (source, source + "_read")]
        return list(sources)

    def approves_skill(self, metadata):
        linux = platform.system() == "Linux"
        title, arguments = metadata.get("title"), metadata.get("arguments")
        if linux and self.pending in ("native_read", "selected_read"):
            if title == "read_file" and arguments == self.pending_read:
                return self.pending
        if self.pending in ("native", "selected"):
            name = self.fixtures["skills"][self.pending]["name"]
            if linux:
                if title == "Skill" and arguments == {"skill": name}:
                    return self.pending
            elif title == "Launch skill: " + name and arguments in (None, {"skill": name}):
                return self.pending
        return None


class Handler(ProviderHandler):
    def do_POST(self):
        probe = self.server.probe
        try:
            require(self.headers.get("Authorization") == "Bearer " + self.server.key,
                    "unauthenticated iFlow model request")
            require(self.path == "/v1/chat/completions", "unexpected iFlow model protocol")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8_000_000, "invalid iFlow request size")
            body = json.loads(self.rfile.read(size))
            delta, finish = probe.response(body)
        except (AssertionError, KeyError, ValueError, TypeError) as error:
            probe.failure = probe.failure or (str(error) if isinstance(error, AssertionError)
                                              else "invalid iFlow Chat Completions envelope")
            self.send_error(422, "probe rejected model request")
            return
        self.send_response(200)
        if body.get("stream"):
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            self.chunk(body["model"], dict(delta, role="assistant"), None)
            self.chunk(body["model"], {}, finish)
            self.wfile.write(b"data: [DONE]\n\n")
        else:
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"id": "chatcmpl-fixture", "object": "chat.completion",
                "model": body["model"], "choices": [{"index": 0, "finish_reason": finish,
                "message": dict(delta, role="assistant")}]}).encode())


def verify_skill_restore(gateway_bin):
    """Use one gateway session, then change the active skill selection."""
    import tempfile
    from urllib.parse import urlencode
    from verify_acp_approval import environment, gateway, request
    from verify_acp_skill_coexistence import (
        AGENT_PROFILE, PROVIDER, interactive_turn, make_skill, model_server, readback,
    )

    with tempfile.TemporaryDirectory(prefix="shiju-iflow-restore-") as directory:
        root = Path(directory).resolve()
        env = environment(root)
        env["ACP_COEXISTENCE_KEY"] = secrets.token_hex(24)
        spec, _ = launch_linux(env)
        workspace = root / "workspace"
        workspace.mkdir()
        selected_parent = workspace / ".shiju/skills"
        fixtures = {"skills": {
            "native": make_skill(workspace / NATIVE_DIR, "native"),
            "selected": make_skill(selected_parent, "selected"),
            "unselected": make_skill(selected_parent, "unselected"),
        }, "mcp": secrets.token_hex(24)}
        query = "?" + urlencode({"directory": str(workspace)})
        model_id = PROVIDER + "/" + MODEL
        with model_server(Handler, env["ACP_COEXISTENCE_KEY"]) as model,                 gateway(str(gateway_bin), root, env) as url:
            request(url, "/global/config", {
                "workspace_dir": str(workspace), "acp": {"iflow": spec},
                "providers": {PROVIDER: {
                    "name": "Private iFlow fixture", "adapter": ADAPTER,
                    "api": f"http://127.0.0.1:{model.server_port}{API_PATH}",
                    "env": ["ACP_COEXISTENCE_KEY"],
                    "models": {MODEL: {
                        "status": "active", "temperature": True, "tool_call": True,
                        "limit": {"context": 100000, "output": 1024},
                    }},
                }},
            }, method="PATCH")
            sid = None
            previous = ""
            for mounted in (True, False, False):
                names = [fixtures["skills"]["selected"]["name"]] if mounted else []
                request(url, "/global/config", {"agents": {AGENT_PROFILE: {
                    "enabled": True, "runtime": {"kind": "acp", "backend": "iflow"},
                    "defaults": {"skills": {"mode": "fixed", "value": names},
                                 "mcp": {"mode": "fixed", "value": []}},
                }}}, method="PATCH")
                if sid is None:
                    sid = json.loads(request(url, "/session" + query, {"title": "iFlow skill restore"}))["id"]
                current = "Inspect fixture resources " + secrets.token_hex(12)
                model.probe = probe = Probe(
                    fixtures, mounted, False, mcp=False, current=current, previous=previous)
                body = {"session_id": sid, "agent": AGENT_PROFILE, "acp_model_source": "session_model",
                        "model": model_id, "options": {"full_access": False, "cwd": str(workspace)},
                        "messages": [{"role": "user", "content": current}]}
                stream, decisions, approvals, access = interactive_turn(url, query, body, probe, False)
                require(not probe.failure, probe.failure)
                events = [json.loads(line[6:]) for line in stream.splitlines() if line.startswith("data: {")]
                types = [event.get("type") for event in events]
                require(types.count("chat.done") == 1 and "chat.error" not in types,
                        "iFlow restore turn did not complete")
                require(decisions == [] and approvals == probe.expected_skill_approvals(),
                        "unexpected approval during iFlow restore")
                require(set(probe.completed) == set(approvals) and probe.requests > len(approvals),
                        "iFlow skill results did not reach the model on the current turn")
                readback(url, query, sid, "iflow", model_id, access)
                previous = current


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Verify iFlow skill changes across a restored session")
    parser.add_argument("--gateway-bin", type=Path, required=True)
    args = parser.parse_args()
    verify_skill_restore(args.gateway_bin.resolve(strict=True))
