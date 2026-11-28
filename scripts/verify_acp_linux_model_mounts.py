#!/usr/bin/env python3
"""Verify reviewed iFlow and GLM Chat tool continuations through the production gateway."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from importlib import import_module
import hashlib
import json
import os
from pathlib import Path
import platform
import secrets
import subprocess
import tempfile
import time
from urllib.parse import urlencode

from acp_model_source_fixture import ModelFixture
from verify_acp_approval import environment, expect_reply_error, gateway, request
from verify_acp_integration import platform_process, require
from verify_acp_model_source import Unavailable
from verify_acp_skill_coexistence import AGENT_PROFILE, PROVIDER, interactive_turn, readback

AGENTS = {
    "iflow": ("iflow-0.5.19-model-source-v4", "read_probe"),
    "glm-acp-agent": ("glm-acp-agent-1.17.1-model-source-v5", "mcp__coexistence__read_probe"),
}
CATALOG = "/project-mcp/definitions"
SERVER = "coexistence"
TOOL = "read_probe"
APPROVAL = f"mcp__{SERVER}__{TOOL}"
MOUNTED_SERVER = "mounted"
STDIO = r'''
count=0
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
            __BEFORE_REPLY__
            count=$((count+1))
            receipt=__RECEIPT__
            result='{"content":[{"type":"text","text":"__CHALLENGE__'"$receipt"'"}]}';;
        *) exit 2;;
    esac
    printf '{"jsonrpc":"2.0","id":%s,"result":%s}\n' "$id" "$result"
done
'''


class TurnApproval:
    def __init__(self, fixture, approvals, pending_request=None):
        self.fixture = fixture
        self.approvals = approvals
        self.pending_request = pending_request
        self.calls = {}

    def approves(self, metadata):
        with self.fixture.lock:
            call = self.fixture.pending_call
        if (not call or call["stage"] not in self.approvals
                or metadata.get("title") != self.approvals[call["stage"]]
                or metadata.get("arguments") != call["input"]):
            return False
        stage = call["stage"]
        require(stage not in self.calls, "duplicate mounted tool approval")
        receipt = {"tool": metadata["title"], "model_call_id": call["id"]}
        if self.pending_request is not None:
            receipt["request_id"] = self.pending_request(metadata)
        self.calls[stage] = receipt
        return True

    def approves_skill(self, metadata):
        # The shared HTTP driver identifies explicit approvals by title and raw input.
        with self.fixture.lock:
            call = self.fixture.pending_call
            if (call and call["stage"] == "native"
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


def replace_session_mount(binary, root, env, workspace, session, mounts):
    log = root / f"session-mount-{len(mounts)}.log"
    with platform_process([str(binary), "acp"], workspace, env, log) as acp:
        acp.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
        loaded, _ = acp.call("session/load", {"sessionId": session, "cwd": str(workspace),
                                             "mcpServers": []})
        revision = loaded["_meta"]["shiju.mounts"]["revision"]
        updated, _ = acp.call("_shiju/session/set_mounts", {"sessionId": session,
            "expectedRevision": revision, "skills": [], "mcpServers": [], "mcpCatalog": mounts})
        require(updated["revision"] == revision + 1, "session mount revision did not advance")
        return updated["revision"]


def process_stat(pid):
    try:
        fields = Path(f"/proc/{pid}/stat").read_bytes().rsplit(b")", 1)[1].split()
        return fields[0], fields[19]
    except (FileNotFoundError, ProcessLookupError):
        return None, None


def blocked_fixture(marker):
    argv = [b"/app/entry", b"-c", b"while IFS= read -r line; do :; done", marker.encode(), b""]
    pids = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            if (entry / "cmdline").read_bytes().split(b"\0") == argv:
                pid = int(entry.name)
                state, start = process_stat(pid)
                if state == b"S":
                    pids.append((pid, start))
        except OSError:
            continue
    require(len(pids) <= 1, "multiple blocked fixture processes")
    return pids[0] if pids else None


def revoke_during_execution(url, query, body, probe, marker, disable):
    with ThreadPoolExecutor(max_workers=1) as pool:
        turn = pool.submit(interactive_turn, url, query, body, probe, False)
        deadline = time.monotonic() + 45
        while not turn.done() and time.monotonic() < deadline:
            blocked = blocked_fixture(marker)
            if blocked is not None:
                break
            time.sleep(0.05)
        else:
            if turn.done():
                turn.result()
            raise AssertionError("stdio tool did not enter its blocking execution")
        pid, start = blocked
        require(process_stat(pid) == (b"S", start) and not turn.done(),
                "tool ended before review withdrawal")
        disable()
        result = turn.result(timeout=8)
        deadline = time.monotonic() + 8
        while process_stat(pid)[1] == start:
            require(time.monotonic() < deadline, "revoked stdio process was not reaped")
            time.sleep(0.05)
        require(blocked_fixture(marker) is None, "revoked stdio tool executed again")
        return result, pid


def observe_turn(url, query, session, agent, model_id, fixture, selected, *,
                 revoked_project=False, both_sources=False, in_flight=None):
    marker = "ACP_MODEL_CONTRACT_" + secrets.token_hex(16)
    with fixture.lock:
        fixture.mount_turn = (marker, tuple(selected))
        fixture.pending_call = None
        offset = len(fixture.records)
    def pending_request(metadata):
        rows = json.loads(request(url, "/permission" + query))
        ids = [row["id"] for row in rows if row["session_id"] == session and row["metadata"] == metadata]
        require(len(ids) == 1, "approval did not identify a unique pending request")
        return ids[0]

    probe = TurnApproval(fixture, selected, pending_request if both_sources or in_flight else None)
    body = {"session_id": session, "agent": AGENT_PROFILE, "acp_model_source": "session_model",
            "model": model_id, "options": {"full_access": False, "cwd": str(Path(fixture.read_path).parent)},
            "messages": [{"role": "user", "content": marker + " Read the fixture resources."}]}
    if in_flight:
        (stream, decisions, native_decisions, access), pid = revoke_during_execution(
            url, query, body, probe, *in_flight)
    else:
        stream, decisions, native_decisions, access = interactive_turn(url, query, body, probe, False)
    events = [json.loads(line[6:]) for line in stream.splitlines() if line.startswith("data: {")]
    types = [event.get("type") for event in events]
    if in_flight:
        errors = [event["error"] for event in events if event.get("type") == "chat.error"]
        require(len(errors) == 1 and "Reviewed ACP MCP selection was revoked" in errors[0]
                and "chat.done" not in types and "FIXTURE_MODEL_REPLY" not in stream,
                "in-flight review withdrawal did not produce the revocation terminal")
        observed = fixture.snapshot()
        requests = observed["requests"][offset:]
        rounds = [row for row in requests if not row.get("auxiliary")]
        require(observed["errors"] == 0 and [row["continued_tools"] for row in rounds] == [[], ["native"]],
                "revoked tool returned or entered a model continuation")
        require(decisions == ["once"] and list(probe.calls) == ["mcp"], "missing blocked tool approval")
        return {"phase": "in_flight_revoked", "session": session, "approval": probe.calls["mcp"],
                "blocked_stdio_pid": pid, "stdio_reaped": True, "relay_requests": requests}
    if revoked_project:
        errors = [event["error"] for event in events if event.get("type") == "chat.error"]
        require(len(errors) == 1 and "reviewed MCP selection: PermissionDenied" in errors[0]
                and "chat.done" not in types, "revoked project selection was not rejected")
        require(not decisions and not native_decisions and len(fixture.snapshot()["requests"]) == offset,
                "revoked project selection reached approval or model dispatch")
        return {"phase": "revoked", "session": session, "mcp_approval": [], "relay_requests": []}
    require(types.count("chat.done") == 1 and "chat.error" not in types,
            "gateway did not complete the model turn")
    answer = "".join(event["delta"] for event in events if event.get("type") == "chat.delta")
    require("FIXTURE_MODEL_REPLY " + fixture.challenge in answer, "model reply did not reach the gateway")
    require(decisions == ["once"] * len(selected) and list(probe.calls) == list(selected),
            "unexpected MCP approval receipt")
    require(native_decisions in ([], ["native_read"]), "unexpected native read approvals")
    observed = fixture.snapshot()
    require(observed["errors"] == 0, "loopback model rejected an upstream request")
    requests = observed["requests"][offset:]
    require(all(row["model"] == fixture.models[0] and row["route"] == fixture.local_route
                and row["authenticated"] is True for row in requests), "unexpected relay model or protocol")
    rounds = [row for row in requests if not row.get("auxiliary")]
    stages = ["native", *selected]
    expected = [stages[:index] for index in range(len(stages) + 1)]
    require([row["continued_tools"] for row in rounds] == expected, "unexpected model continuation sequence")
    ids = rounds[-1]["continued_call_ids"]
    require(len(set(ids)) == len(ids)
            and ids[1:] == [receipt["model_call_id"] for receipt in probe.calls.values()],
            "approved tool did not continue with its issued model call ID")
    # A fast resumed turn may close its private ACP process before live access polling.
    require_live_access = "session" in selected if both_sources else bool(selected)
    selection = readback(url, query, session, agent, model_id, access,
                         require_live_access=require_live_access)
    row = {"phase": "mounted" if selected else "removed", "session": session,
            "source": selection["source"], "selected_model": selection["model"],
            "live_access_observed": bool(selection["access"]),
            "mcp_approval": decisions, "relay_requests": requests}
    if both_sources:
        row.update(phase="both" if "session" in selected else "project_after_removal",
                   approvals=probe.calls,
                   executions={source: rounds[-1]["continued_tools"].count(source)
                               for source in ("project", "session")})
    else:
        row["approved_model_call_id"] = probe.calls.get("mcp", {}).get("model_call_id")
    return row


def run(agent, binary, report, session_mount=False, both_sources=False, in_flight=False):
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
        executable_hash = hashlib.sha256(executable.read_bytes()).hexdigest()
        block_marker = "shiju-acp-block-" + secrets.token_hex(16) if in_flight else None

        def connection(tool, challenge, counted=False):
            block = (f"exec /app/entry -c 'while IFS= read -r line; do :; done' {block_marker}"
                     if block_marker else ":")
            script = (STDIO.replace(TOOL, tool).replace("__CHALLENGE__", challenge)
                      .replace("__BEFORE_REPLY__", block)
                      .replace("__RECEIPT__", '"|execution=$count|"' if counted else '""'))
            return {"transport": "stdio", "command": str(executable), "args": ["-c", script]}

        if both_sources:
            services = {}
            approvals = {}
            for source, server, tool in (("project", SERVER, "read_project"),
                                         ("session", MOUNTED_SERVER, "read_mounted")):
                challenge = secrets.token_hex(24)
                services[server] = (tool, connection(tool, challenge, counted=True))
                approvals[source] = f"mcp__{server}__{tool}"
                name = tool if agent == "iflow" else approvals[source]
                fixture.mount_tools[source] = (name, challenge + "|execution=1|")
        else:
            services = {SERVER: (TOOL, connection(TOOL, fixture.mcp_challenge))}
            approvals = {"mcp": APPROVAL}
        project_connection = services[SERVER][1]
        (workspace / ".vwacprc.json").write_text(json.dumps({"mcpServers": [{
            "name": SERVER, "command": project_connection["command"],
            "args": project_connection["args"], "env": [],
        }]}))
        if agent == "iflow" and not session_mount:
            # iFlow cross-checks project-source names against its native declaration.
            native_settings = workspace / ".iflow/settings.json"
            native_settings.parent.mkdir()
            native_settings.write_text(json.dumps({"mcpServers": {SERVER: {}}}))
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
            select_profile(url, agent, not session_mount)
            session = json.loads(request(url, "/session" + query, {"title": "ACP model mounts"}))
            review_policy(policy, Path(env["HOME"]), token, session["projectID"])
            report["stage"] = "catalog"
            records = {}

            def review(server, action):
                record = records[server]
                response = json.loads(request(url, f"{CATALOG}/{server}", {
                    "projectId": session["projectID"], "expectedRevision": record["revision"],
                    "linuxStdioSelection": action,
                }, method="PUT", extra_headers=headers))
                require(response["record"] == record, "review changed the selected definition")

            for server, (tool, definition) in services.items():
                records[server] = json.loads(request(url, CATALOG, {
                    "projectId": session["projectID"], "id": server, "connection": definition,
                }, extra_headers=headers))["record"]
                review(server, {"action": "select", "sha256": executable_hash, "allowedTools": [tool]})
            if in_flight:
                report["stage"] = "in_flight"
                revoked = observe_turn(url, query, session["id"], agent, model_id, fixture, approvals,
                                       in_flight=(block_marker, lambda: review(SERVER, {"action": "disable"})))
                report["turns"].append(revoked)
                report["stage"] = "after_revocation"
                old_approval = revoked["approval"]["request_id"]
                expect_reply_error(url, f"/permission/{old_approval}/reply" + query, "once", 404)
                report["removed_approval_replay_status"] = 404
                report["turns"].append(observe_turn(url, query, session["id"], agent, model_id, fixture, {},
                                                    revoked_project=True))
                require(blocked_fixture(block_marker) is None, "old approval reached stdio")
                return
            if session_mount or both_sources:
                scope = {"kind": "project", "scope": {"platform_id": "10000",
                    "company_id": "100000000", "project_id": session["projectID"]}}
                mounted_record = records[MOUNTED_SERVER if both_sources else SERVER]
                mounted_revision = replace_session_mount(binary, root, env, workspace, session["id"],
                                                         [{"scope": scope, "record": mounted_record}])
            report["stage"] = "mounted"
            mounted = observe_turn(url, query, session["id"], agent, model_id, fixture, approvals,
                                   both_sources=both_sources)
            report["turns"].append(mounted)
            report["stage"] = "removed"
            if session_mount or both_sources:
                mounted["mount_revision"] = mounted_revision
                removed_revision = replace_session_mount(binary, root, env, workspace, session["id"], [])
                require(removed_revision == mounted_revision + 1, "removal did not follow the mounted revision")
            else:
                review(SERVER, {"action": "disable"})
            if both_sources:
                old_approval = mounted["approvals"]["session"]["request_id"]
                expect_reply_error(url, f"/permission/{old_approval}/reply" + query, "once", 404)
                report["removed_approval_replay_status"] = 404
            removed = observe_turn(url, query, session["id"], agent, model_id, fixture,
                                   {"project": approvals["project"]} if both_sources else {},
                                   revoked_project=not (session_mount or both_sources), both_sources=both_sources)
            report["turns"].append(removed)
            if session_mount or both_sources:
                removed["mount_revision"] = removed_revision
            mounted_ids, removed_ids = (
                {call_id for row in turn["relay_requests"] for call_id in row.get("continued_call_ids", ())}
                for turn in report["turns"])
            require(mounted_ids.isdisjoint(removed_ids), "removed turn reused a mounted turn call ID")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agent", choices=AGENTS, required=True)
    parser.add_argument("--gateway-bin", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    mounts = parser.add_mutually_exclusive_group()
    mounts.add_argument("--session-mount", action="store_true",
                        help="select and remove a committed ACP session mount")
    mounts.add_argument("--both-sources", action="store_true",
                        help="select reviewed project and session services together, then remove the session service")
    mounts.add_argument("--in-flight-revocation", action="store_true",
                        help="withdraw project stdio review after the approved call enters execution")
    args = parser.parse_args()
    report = {"agent": args.agent, "platform": f"{platform.system()}-{platform.machine()}",
              "mount_source": "both" if args.both_sources else "session" if args.session_mount else "project",
              "in_flight_revocation": args.in_flight_revocation,
              "turns": [], "passed": False}
    try:
        run(args.agent, args.gateway_bin.resolve(strict=True), report,
            args.session_mount, args.both_sources, args.in_flight_revocation)
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
