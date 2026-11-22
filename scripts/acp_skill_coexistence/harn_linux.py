"""Exercise the Linux Harn source build through an isolated production gateway."""
from contextlib import ExitStack
import hashlib
import json
import os
from pathlib import Path
import platform
import secrets
import shutil
import sys
import tempfile
from urllib.parse import urlencode

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from acp_skill_coexistence import harn
from verify_acp_approval import GatewayError, environment, gateway, request
from verify_acp_integration import require
from verify_acp_model_source import ROOT
from verify_acp_skill_coexistence import (
    AGENT_PROFILE, PROVIDER, interactive_turn, make_skill, model_server, readback,
)


def staged_runtime(env):
    contract = json.loads((ROOT / "patches/acp-model-source/harn/linux.json").read_text())
    target = f"{platform.machine()}-unknown-linux-gnu"
    require(platform.system() == "Linux" and target in contract["targets"], "unreviewed Harn target")
    source = Path(os.environ["SHIJU_ACP_HARN_RUNTIME"]).resolve(strict=True)
    runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / contract["directory"]
    runtime.mkdir(parents=True, mode=0o700)
    receipt = json.loads((source / "build-receipt.json").read_text())
    for field in ("version", "source_sha256", "patch_sha256"):
        require(receipt[field] == contract[field], f"unexpected Harn {field}")
    require(receipt["target"] == target, "unexpected Harn receipt target")
    for name, digest in {**contract["files"], "harn-native": receipt["engine_sha256"]}.items():
        shutil.copy2(source / name, runtime / name)
        checksum = hashlib.sha256()
        with (runtime / name).open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                checksum.update(block)
        require(checksum.hexdigest() == digest, f"Harn {name} checksum mismatch")
    shutil.copyfile(source / "build-receipt.json", runtime / "build-receipt.json")
    print(json.dumps({"target": target, "entry": str(runtime / "harn"),
                      "engine_sha256": receipt["engine_sha256"], "source": "session_model",
                      "protocol": "Chat", "version": receipt["version"]}), flush=True)
    return runtime / "harn"



class WriteProbe(harn.Probe):
    def __init__(self, fixtures, path, rejected):
        super().__init__(fixtures, False, False)
        self.rejected = rejected
        self.arguments = {"path": str(path), "content": secrets.token_hex(24)}

    def response(self, body):
        self.requests += 1
        require(body["model"] == harn.MODEL and body["stream"] is True, "unexpected Harn write model")
        if self.requests == 1:
            tools = {tool["function"]["name"]: tool["function"] for tool in body["tools"]}
            return self.call(tools, "write", "write_file", self.arguments)
        _, call_id = self.pending
        replies = [item["content"] for item in body["messages"]
                   if item["role"] == "tool" and item.get("tool_call_id") == call_id]
        if self.rejected:
            require(len(replies) == 1 and "File written" not in str(replies[0]),
                    "denied Harn write returned a success result")
            return {"content": "WRITE_DENIED"}, "stop"
        require(len(replies) == 1 and "File written" in str(replies[0]),
                "missing approved Harn write continuation")
        self.completed["write"] = call_id
        return {"content": "WRITE_DONE"}, "stop"

    def approves(self, metadata):
        return metadata.get("title") == "Harn write_file" and metadata.get("arguments") == self.arguments


def run(binary):
    require(os.geteuid() != 0, "run the Harn probe as a non-root Linux user")
    with tempfile.TemporaryDirectory(prefix="shiju-harn-linux-") as directory, ExitStack() as stack:
        root = Path(directory).resolve()
        env = environment(root)
        entry = staged_runtime(env)
        workspace = root / "workspace"
        workspace.mkdir()
        fixtures = {"skills": {
            "native": make_skill(workspace / harn.NATIVE_DIR, "native"),
            "selected": make_skill(workspace / ".shiju/skills", "selected"),
        }, "mcp": secrets.token_hex(24)}
        harn.prepare_workspace(workspace)
        audit = root / "mcp-audit"
        # This fixture audits even process startup, before MCP initialization.
        service = workspace / "mcp_probe.py"
        service.write_text("from pathlib import Path\nimport sys\n"
                           "with Path(sys.argv[1]).open('a') as audit:\n    audit.write('started\\n')\n")
        (workspace / ".vwacprc.json").write_text(json.dumps({"mcpServers": [{
            "name": "coexistence", "command": str(Path(sys.executable).resolve()),
            "args": [str(service), str(audit)], "env": [],
        }]}))
        env["ACP_COEXISTENCE_KEY"] = secrets.token_hex(24)
        model = stack.enter_context(model_server(harn.Handler, env["ACP_COEXISTENCE_KEY"]))
        url = stack.enter_context(gateway(str(binary), root, env))
        spec = json.loads(request(url, "/global/config/acp/defaults"))["harn"]
        require(Path(spec["command"]) == entry and not spec.get("args"), "gateway did not discover managed Linux Harn")
        model_id = f"{PROVIDER}/{harn.MODEL}"
        request(url, "/global/config", {
            "workspace_dir": str(workspace), "acp": {"harn": spec},
            "providers": {PROVIDER: {"name": "Private Harn fixture",
                "api": f"http://127.0.0.1:{model.server_port}/v1", "adapter": "openai",
                "env": ["ACP_COEXISTENCE_KEY"], "models": {harn.MODEL: {
                    "status": "active", "tool_call": True,
                    "limit": {"context": 100000, "output": harn.MODEL_OUTPUT_LIMIT},
                }}}},
        }, method="PATCH")
        query = "?" + urlencode({"directory": str(workspace)})
        for phase in ("baseline", "selected", "blocked", "removed", "approved-write", "denied-write"):
            selected = phase in ("selected", "blocked")
            blocked = phase == "blocked"
            request(url, "/global/config", {"agents": {AGENT_PROFILE: {
                "enabled": True, "runtime": {"kind": "acp", "backend": "harn"},
                "defaults": {
                    "skills": {"mode": "fixed", "value": [fixtures["skills"]["selected"]["name"]] if selected else []},
                    "mcp": {"mode": "fixed", "value": ["coexistence"] if blocked else []},
                },
            }}}, method="PATCH")
            sid = json.loads(request(url, "/session" + query, {"title": phase}))["id"]
            write = phase.endswith("-write")
            rejected = phase == "denied-write"
            path = workspace / f"{phase}.txt"
            model.probe = probe = (WriteProbe(fixtures, path, rejected) if write
                                   else harn.Probe(fixtures, False, False, selected=selected))
            body = {"session_id": sid, "agent": AGENT_PROFILE, "acp_model_source": "session_model",
                    "model": model_id, "options": {"full_access": False, "cwd": str(workspace)},
                    "messages": [{"role": "user", "content": "Read the available fixture skills."}]}
            if blocked:
                try:
                    stream = request(url, "/chat/stream" + query, body)
                except GatewayError as error:
                    require("MCP" in str(error), "expected an explicit gateway MCP rejection")
                else:
                    events = [json.loads(line[6:]) for line in stream.splitlines() if line.startswith("data: {")]
                    require(any(event.get("type") == "chat.error"
                                and "MCP" in str(event.get("error")) for event in events)
                            and not any(event.get("type") == "chat.done" for event in events),
                            "expected an explicit gateway MCP rejection")
                require(probe.requests == 0, "blocked MCP selection reached the model")
            elif write:
                stream, decisions, _, _ = interactive_turn(url, query, body, probe, rejected)
                require(not probe.failure and decisions == ["reject" if rejected else "once"],
                        "Harn write lacked one explicit host decision")
                if rejected:
                    require(not path.exists() and not probe.completed, "denied Harn write had an effect")
                else:
                    require(path.read_text() == probe.arguments["content"] and probe.completed.keys() == {"write"}
                            and '"chat.done"' in stream, "Harn did not execute and return the approved write")
            else:
                stream, decisions, _, live = interactive_turn(url, query, body, probe, False)
                events = [json.loads(line[6:]) for line in stream.splitlines() if line.startswith("data: {")]
                types = [event.get("type") for event in events]
                require(types.count("chat.done") == 1 and "chat.error" not in types and not decisions,
                        f"Harn {phase} failed to complete")
                expected = {"native", "native_read"} | ({"selected", "selected_read"} if selected else set())
                require(not probe.failure and probe.completed.keys() == expected, "Harn skill continuation mismatch")
                readback(url, query, sid, "harn", model_id, live)
            require(not audit.exists(), f"Harn started an unverified MCP service during {phase}")


if __name__ == "__main__":
    run(Path(sys.argv[1]).resolve(strict=True))
