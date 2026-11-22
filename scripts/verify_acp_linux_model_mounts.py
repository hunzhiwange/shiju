#!/usr/bin/env python3
"""Verify reviewed iFlow and GLM Chat tool continuations through the production gateway."""
import argparse
from importlib import import_module
import hashlib
import json
import os
from pathlib import Path
import platform
import secrets
import subprocess
import tempfile
from urllib.parse import urlencode

from acp_model_source_fixture import ModelFixture
from verify_acp_approval import environment, gateway, request
from verify_acp_integration import require
from verify_acp_model_source import Unavailable
from verify_acp_skill_coexistence import AGENT_PROFILE, PROVIDER, interactive_turn, readback

AGENTS = {
    "iflow": ("iflow-0.5.19-model-source-v4", "read_probe"),
    "glm-acp-agent": ("glm-acp-agent-1.17.1-model-source-v5", "mcp__coexistence__read_probe"),
}
CATALOG = "/agent-apps/mcp"
SERVER = "coexistence"
TOOL = "read_probe"
APPROVAL = f"mcp__{SERVER}__{TOOL}"
STDIO = r'''
while IFS= read -r line; do
    case "$line" in
        *'"id":'*) id=${line#*\"id\":}; id=${id%%,*}; id=${id%%\}*};;
        *) continue;;
    esac
    case "$line" in
        *'"method":"initialize"'*)
            result='{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}}';;
        *'"method":"tools/list"'*)
            result='{"tools":[{"name":"read_probe","inputSchema":{"type":"object","properties":{}}}]}';;
        *'"method":"tools/call"'*)
            case "$line" in *'"shijuCallId"'*) exit 3;; esac
            result='{"content":[{"type":"text","text":"__CHALLENGE__"}]}';;
        *) exit 2;;
    esac
    printf '{"jsonrpc":"2.0","id":%s,"result":%s}\n' "$id" "$result"
done
'''


class TurnApproval:
    def __init__(self, fixture):
        self.fixture = fixture
        self.mcp_call = None

    def approves(self, metadata):
        with self.fixture.lock:
            call = self.fixture.pending_call
            if (call and call["name"] == self.fixture.mcp_tool
                    and metadata.get("title") == APPROVAL
                    and metadata.get("arguments") == call["input"]):
                require(self.mcp_call is None, "duplicate mounted tool approval")
                self.mcp_call = call["id"]
                return True
        return False

    def approves_skill(self, metadata):
        # The shared HTTP driver identifies explicit approvals by title and raw input.
        with self.fixture.lock:
            call = self.fixture.pending_call
            if (call and call["name"] != self.fixture.mcp_tool
                    and metadata.get("title") == call["name"]
                    and metadata.get("arguments") == call["input"]):
                return "native_read"
        return None


def review_policy(policy, home, token, project):
    policy.write_text(json.dumps({
        "hosts": [{"id": "desktop-local", "binding": "local", "online": True,
                   "home": str(home), "applications": str(home), "search_paths": []}],
        "principals": [{"id": "acceptance", "token_sha256": hashlib.sha256(token.encode()).hexdigest(),
                        "grants": [{"platform_id": "10000", "company_id": "100000000",
                                    "host_id": "desktop-local", "provider_config_write": True,
                                    "mcp_catalog_write_projects": [project]}]}],
    }))
    policy.chmod(0o600)


def select_profile(url, agent, mounted):
    profile = {"enabled": True, "runtime": {"kind": "acp", "backend": agent},
               "defaults": {"skills": {"mode": "fixed", "value": []},
                            "mcp": {"mode": "fixed", "value": [SERVER] if mounted else []}}}
    request(url, "/global/config", {"agents": {AGENT_PROFILE: profile}}, method="PATCH")


def observe_turn(url, query, session, agent, model_id, fixture, mounted):
    marker = "ACP_MODEL_CONTRACT_" + secrets.token_hex(16)
    with fixture.lock:
        fixture.mount_turn = (marker, mounted)
        fixture.pending_call = None
        offset = len(fixture.records)
    probe = TurnApproval(fixture)
    body = {"session_id": session, "agent": AGENT_PROFILE, "acp_model_source": "session_model",
            "model": model_id, "options": {"full_access": False, "cwd": str(Path(fixture.read_path).parent)},
            "messages": [{"role": "user", "content": marker + " Read the fixture resources."}]}
    stream, decisions, native_decisions, access = interactive_turn(url, query, body, probe, False)
    events = [json.loads(line[6:]) for line in stream.splitlines() if line.startswith("data: {")]
    types = [event.get("type") for event in events]
    require(types.count("chat.done") == 1 and "chat.error" not in types,
            "gateway did not complete the model turn")
    answer = "".join(event["delta"] for event in events if event.get("type") == "chat.delta")
    require("FIXTURE_MODEL_REPLY " + fixture.challenge in answer, "model reply did not reach the gateway")
    require(decisions == (["once"] if mounted else []), "unexpected MCP approval receipt")
    require(native_decisions in ([], ["native_read"]), "unexpected native read approvals")
    observed = fixture.snapshot()
    require(observed["errors"] == 0, "loopback model rejected an upstream request")
    requests = observed["requests"][offset:]
    require(all(row["model"] == fixture.models[0] and row["route"] == fixture.local_route
                and row["authenticated"] is True for row in requests), "unexpected relay model or protocol")
    rounds = [row for row in requests if not row.get("auxiliary")]
    expected = [[], ["native"], ["native", "mcp"]] if mounted else [[], ["native"]]
    require([row["continued_tools"] for row in rounds] == expected, "unexpected model continuation sequence")
    ids = rounds[-1]["continued_call_ids"]
    require(len(set(ids)) == len(ids) and (not mounted or ids[-1] == probe.mcp_call),
            "approved tool did not continue with its issued model call ID")
    selection = readback(url, query, session, agent, model_id, access)
    return {"phase": "mounted" if mounted else "removed", "session": session,
            "source": selection["source"], "selected_model": selection["model"],
            "mcp_approval": decisions, "approved_model_call_id": probe.mcp_call, "relay_requests": requests}


def run(agent, binary, report):
    require(platform.system() == "Linux" and platform.machine() in ("x86_64", "aarch64")
            and os.geteuid() != 0, "requires non-root Linux x86_64 or aarch64")
    supplier = import_module("acp_skill_coexistence." + agent.replace("-", "_"))
    revision, model_tool = AGENTS[agent]
    with tempfile.TemporaryDirectory(prefix="shiju-acp-chat-mounts-") as directory, ModelFixture(local_protocol="chat") as fixture:
        root = Path(directory).resolve()
        env = environment(root)
        report["stage"] = "artifact"
        spec, report["artifact"] = supplier.launch(env)
        require(report["artifact"]["revision"] == revision, "unexpected reviewed adapter revision")
        workspace = root / "workspace"
        workspace.mkdir()
        fixture.models = (supplier.MODEL,)
        fixture.mcp_tool = model_tool
        fixture.read_path = str(workspace / "challenge.txt")
        Path(fixture.read_path).write_text(fixture.challenge)
        executable = Path("/bin/dash").resolve(strict=True)
        args = ["-c", STDIO.replace("__CHALLENGE__", fixture.mcp_challenge)]
        connection = {"transport": "stdio", "command": str(executable), "args": args}
        (workspace / ".vwacprc.json").write_text(json.dumps({"mcpServers": [{
            "name": SERVER, "command": str(executable), "args": args, "env": [],
        }]}))
        policy = root / "access.json"
        env.update(SHIJU_AGENT_APPS_ACCESS_FILE=str(policy), ACP_COEXISTENCE_KEY=fixture.key)
        token = secrets.token_urlsafe(32)
        headers = {"X-Agent-Apps-Token": token}
        query = "?" + urlencode({"directory": str(workspace)})
        model_id = PROVIDER + "/" + supplier.MODEL
        report["stage"] = "gateway"
        with gateway(str(binary), root, env, cwd=workspace) as url:
            request(url, "/global/config", {"workspace_dir": str(workspace), "acp": {agent: spec},
                "providers": {PROVIDER: {"api": fixture.url + supplier.API_PATH, "adapter": supplier.ADAPTER,
                    "env": ["ACP_COEXISTENCE_KEY"],
                    "models": {supplier.MODEL: {"status": "active", "temperature": True, "tool_call": True,
                        "limit": {"context": 100000, "output": getattr(supplier, "MODEL_OUTPUT_LIMIT", 1024)}}}}}},
                method="PATCH")
            select_profile(url, agent, True)
            session = json.loads(request(url, "/session" + query, {"title": "ACP model mounts"}))
            review_policy(policy, Path(env["HOME"]), token, session["projectID"])
            report["stage"] = "catalog"
            record = json.loads(request(url, CATALOG, {"projectId": session["projectID"], "id": SERVER,
                                                       "connection": connection}, extra_headers=headers))["record"]

            def review(action):
                response = json.loads(request(url, f"{CATALOG}/{SERVER}", {
                    "projectId": session["projectID"], "expectedRevision": record["revision"],
                    "linuxStdioSelection": action,
                }, method="PUT", extra_headers=headers))
                require(response["record"] == record, "review changed the selected definition")

            review({"action": "select", "sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
                    "allowedTools": [TOOL]})
            report["stage"] = "mounted"
            report["turns"].append(observe_turn(url, query, session["id"], agent, model_id, fixture, True))
            report["stage"] = "removed"
            review({"action": "disable"})
            select_profile(url, agent, False)
            report["turns"].append(observe_turn(url, query, session["id"], agent, model_id, fixture, False))
            mounted_ids, removed_ids = (
                {call_id for row in turn["relay_requests"] for call_id in row.get("continued_call_ids", ())}
                for turn in report["turns"])
            require(mounted_ids.isdisjoint(removed_ids), "removed turn reused a mounted turn call ID")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agent", choices=AGENTS, required=True)
    parser.add_argument("--gateway-bin", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = {"agent": args.agent, "platform": f"{platform.system()}-{platform.machine()}",
              "turns": [], "passed": False}
    try:
        run(args.agent, args.gateway_bin.resolve(strict=True), report)
        report.pop("stage", None)
        report["passed"] = True
    except AssertionError as error:
        report["failure"] = str(error)
    except (OSError, ValueError, KeyError, TypeError, Unavailable, subprocess.SubprocessError) as error:
        report["failure"] = type(error).__name__
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    return int(not report["passed"])


if __name__ == "__main__":
    raise SystemExit(main())
