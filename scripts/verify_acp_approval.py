#!/usr/bin/env python3
"""Verify ACP approval through the desktop chat HTTP route in an isolated gateway."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from contextlib import ExitStack, contextmanager
import json
import os
from pathlib import Path
import platform
import signal
import socket
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import ProxyHandler, Request, build_opener

from acp_approval_model import CONTENTS as CONTENT, DONE, matches_workspace_edit, model_fixture
from acp_skill_mount_codex import locate_codex
from verify_acp_model_source import (
    ROOT, Unavailable, cargo_slot, isolated_environment, locate, openclaw_native_gateway,
)

MANIFEST = ROOT / "docs/shiju/acp-model-source/cli-versions.json"
PHASES = (("approval", False), ("full_access", True), ("revoked", False))
HTTP = build_opener(ProxyHandler({}))


class GatewayError(Unavailable):
    def __init__(self, status, route, detail):
        super().__init__(f"{route} HTTP {status}: {detail}")
        self.status = status


def require(condition, reason):
    if not condition:
        raise AssertionError(reason)


def send(message):
    print(json.dumps({"jsonrpc": "2.0", **message}), flush=True)


def peer(workspace, outside):
    """Controlled custom ACP backend. It can only mutate files through ACP callbacks."""
    def call(sequence, method, **params):
        send({"id": sequence, "method": method, "params": {"sessionId": "approval-peer", **params}})
        return json.loads(sys.stdin.readline())

    lifecycle = None
    initial_workspace = workspace
    initial_outside = outside
    for line in sys.stdin:
        frame = json.loads(line)
        if "id" not in frame:
            continue
        method = frame["method"]
        if method in ("session/new", "session/load"):
            workspace = Path(frame["params"]["cwd"])
            require(workspace.parent == initial_workspace.parent, "unexpected fixture workspace")
            outside = initial_outside if workspace == initial_workspace else workspace.with_suffix(".outside")
        if method == "initialize":
            result = {"protocolVersion": 1, "agentCapabilities": {"loadSession": True},
                      "agentInfo": {"name": "approval-fixture", "version": "1"}}
        elif method == "session/new":
            lifecycle = "new"
            result = {"sessionId": "approval-peer"}
        elif method == "session/load":
            lifecycle = "load"
            result = {}
        elif method == "session/prompt":
            # This peer is a protocol fixture. Supplier rows use a real adapter.
            for sequence, content in enumerate(CONTENT, 100):
                response = call(sequence, "fs/write_text_file", path=str(workspace / "note.txt"),
                                content=content)
                require("result" in response, "workspace write denied")
            options = [{"optionId": key, "name": key, "kind": kind} for key, kind in
                       (("always", "allow_always"), ("once", "allow_once"), ("deny", "reject_once"))]
            decisions = []
            for sequence, kind in enumerate(("edit", "delete", "execute"), 102):
                response = call(sequence, "session/request_permission", options=options,
                                toolCall={"toolCallId": str(sequence), "title": kind, "kind": kind})
                decisions.append(response.get("result", {}).get("outcome", {}).get("optionId"))
            response = call(105, "fs/write_text_file", path=str(outside), content="outside edit")
            send({"method": "session/update", "params": {"sessionId": "approval-peer", "update": {
                "sessionUpdate": "agent_message_chunk", "content": {
                    "type": "text", "text": DONE + " " + json.dumps([
                        decisions, "result" in response, lifecycle
                    ])
                }
            }}})
            result = {"stopReason": "end_turn"}
        else:
            send({"id": frame["id"], "error": {"code": -32601, "message": "Method not found"}})
            continue
        send({"id": frame["id"], "result": result})


def environment(root):
    allowed = ("PATH", "LANG", "SystemRoot", "CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR")
    env = {key: os.environ[key] for key in allowed if key in os.environ}
    # The child gets an isolated account/config scope. The parent environment is untouched.
    for key, name in (("HOME", "home"), ("VIBEWINDOW_CONFIG_DIR", "config"), ("TMPDIR", "tmp")):
        directory = root / name
        directory.mkdir(mode=0o700)
        env[key] = str(directory)
    return env


def request(url, path, body=None, method=None):
    data = None if body is None else json.dumps(body).encode()
    headers = {"Content-Type": "application/json", "X-Platform-ID": "10000",
               "X-Company-ID": "100000000"}
    try:
        with HTTP.open(Request(url + path, data, headers, method=method), timeout=60) as response:
            return response.read(2_000_000).decode()
    except HTTPError as error:
        payload = error.read(1024).decode(errors="replace")
        raise GatewayError(error.code, path.split("?")[0], payload) from error


def expect_reply_error(url, route, reply, status):
    try:
        request(url, route, {"reply": reply})
    except GatewayError as error:
        require(error.status == status, f"unexpected approval reply status {error.status}")
    else:
        raise AssertionError("an invalid permission reply was accepted")


def interactive_turn(url, query, other_query, body, workspace, approve_edits, rendezvous=None,
                     gemini_write_expected=None):
    """Drive the same one-time approval route used by the desktop."""
    decisions = []
    seen = set()
    with ThreadPoolExecutor(max_workers=1) as pool:
        stream = pool.submit(request, url, "/chat/stream" + query, body)
        while not stream.done():
            pending = json.loads(request(url, "/permission" + query))
            for row in pending:
                if row["session_id"] != body["session_id"] or row["id"] in seen:
                    continue
                metadata = row["metadata"]
                require(metadata.get("requested_channel") == "desktop-acp",
                        "permission request does not belong to ACP")
                arguments = metadata.get("arguments") or {}
                # Only approve an exact fixture write. Titles do not authorize commands.
                write = matches_workspace_edit(arguments, workspace)
                # Gemini omits rawInput from permission requests. In this isolated
                # single-tool turn the fixture itself selected the exact file input.
                if gemini_write_expected and gemini_write_expected() and not arguments:
                    write = metadata.get("title") in ("Writing to note.txt", "Confirm Write: note.txt")
                decision = "once" if approve_edits and write else "reject"
                if rendezvous is not None and not decisions:
                    rendezvous.wait(timeout=20)
                route = f"/permission/{row['id']}/reply"
                foreign = json.loads(request(url, "/permission" + other_query))
                require(all(candidate["id"] != row["id"] for candidate in foreign),
                        "another workspace can see the pending approval")
                expect_reply_error(url, route + other_query, "once", 404)
                expect_reply_error(url, route + query, "always", 400)
                require(json.loads(request(url, route + query, {"reply": decision})) is True,
                        "permission reply failed")
                expect_reply_error(url, route + query, decision, 404)
                decisions.append({"workspace_edit": write, "reply": decision})
                seen.add(row["id"])
            time.sleep(0.05)
        return stream.result(), decisions


def stream_evidence(stream):
    events = [json.loads(line.removeprefix("data: "))
              for line in stream.splitlines() if line.startswith("data: {")]
    types = {event.get("type") for event in events}
    answer = "".join(event["delta"] for event in events if event.get("type") == "chat.delta")
    return DONE in answer and "chat.done" in types and "chat.error" not in types, answer


def stream_failure_categories(stream):
    categories = {
        "opencode_requires_full_access": ("opencode approval mode cannot constrain native tools",),
        "authentication": ("auth", "api key", "credential"),
        "model_source": ("model", "provider"),
        "reviewed_runtime": ("reviewed", "checksum", "artifact"),
        "recovery": ("resume", "session load", "recover"),
        "permission_control": ("permissions", "permission control", "restricted mode"),
        "transport": ("connection", "response", "timed out"),
    }
    errors = [json.loads(line.removeprefix("data: ")).get("error") for line in stream.splitlines()
              if line.startswith("data: {") and '"chat.error"' in line]
    return [category for category, terms in categories.items()
            if any(term in str(errors).lower() for term in terms)] or ["missing_turn_completion"]


def parallel_host_sessions(url, agent, workspaces):
    queries = ["?" + urlencode({"directory": str(workspace)}) for workspace in workspaces]
    barrier = threading.Barrier(len(workspaces))

    def run(index):
        query = queries[index]
        workspace = workspaces[index]
        session = json.loads(request(url, "/session" + query, {"title": "Parallel ACP approval"}))["id"]
        body = {"session_id": session, "agent": agent, "acp_model_source": "cli_local",
                "messages": [{"role": "user", "content": "Run the parallel approval fixture"}],
                "options": {"full_access": False, "cwd": str(workspace),
                            "acp_close_client_after_request": True}}
        stream, decisions = interactive_turn(
            url, query, queries[1 - index], body, workspace, False, barrier
        )
        completed, answer = stream_evidence(stream)
        result = answer.partition(DONE + " ")[2].strip()
        require(completed and json.loads(result) == [["deny"] * 3, False, "new"],
                "parallel session did not finish with its own permission results")
        require(len(decisions) == 3, "an approval escaped or replayed during parallel turns")
        return len(decisions)

    with ThreadPoolExecutor(max_workers=len(workspaces)) as pool:
        counts = list(pool.map(run, range(len(workspaces))))
    return {"sessions": len(counts), "permission_events": sum(counts),
            "cross_workspace_replies_rejected": sum(counts), "replays_rejected": sum(counts)}


def stop_gateway_children(session_id):
    # ACP creates separate process groups for individual adapters. They still
    # share the gateway's private OS session, even after the gateway exits.
    for signum in (signal.SIGTERM, signal.SIGKILL):
        for value in subprocess.check_output(["ps", "-axo", "pid="], text=True).split():
            pid = int(value)
            try:
                if os.getsid(pid) == session_id:
                    os.kill(pid, signum)
            except (ProcessLookupError, PermissionError):
                pass
        if signum == signal.SIGTERM:
            time.sleep(0.1)


@contextmanager
def gateway(binary, root, env):
    db = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "database/vibe-window.sqlite3"
    db.parent.mkdir()
    with sqlite3.connect(db) as conn:
        conn.execute("CREATE TABLE system_configs (platform_id TEXT, company_id TEXT, "
                     "payload_json TEXT, updated_at_ms INTEGER, PRIMARY KEY(platform_id,company_id))")
        conn.execute("INSERT INTO system_configs VALUES ('10000','100000000',?,1)",
                     (json.dumps({"gateway": {"auth_enabled": False}}),))
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    url = f"http://127.0.0.1:{port}/v1"
    with (root / "gateway.log").open("wb") as log:
        child = subprocess.Popen([binary, "gateway", "--host", "127.0.0.1", "-p", str(port)],
                                 cwd=root, env=env, stdout=log, stderr=log, start_new_session=True)
        try:
            for _ in range(120):
                if child.poll() is not None:
                    raise Unavailable(f"isolated gateway exited during startup ({child.returncode})")
                try:
                    request(url, "/global/health")
                    break
                except URLError:
                    time.sleep(0.25)
            else:
                raise Unavailable("isolated gateway did not become ready")
            yield url
        finally:
            try:
                os.killpg(child.pid, signal.SIGINT)
            except ProcessLookupError:
                pass
            try:
                child.wait(timeout=8)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
            stop_gateway_children(child.pid)


def session_turns(binary, backend, source, row, entry=None):
    with tempfile.TemporaryDirectory(prefix="shiju-acp-approval-") as name:
        root = Path(name).resolve()
        env = environment(root)
        workspace = root / "workspace"
        workspace.mkdir()
        outside = root / "outside.txt"
        agent = "approval-fixture"
        with model_fixture(workspace, outside) as fixture, ExitStack() as owned:
            if entry is None:
                spec = {"command": sys.executable, "args": [
                    str(Path(__file__).resolve()), "--peer", str(workspace), str(outside)
                ], "env": {}}
            else:
                spec = locate_codex(entry, env) if backend == "codex" else locate(entry, env)
                if backend == "gemini":
                    spec["args"] = ["--experimental-acp"]
                if backend == "qwen":
                    spec["args"] = ["--acp", "--experimental-skills"]
                if source == "cli_local":
                    native = root / "native"
                    native.mkdir(mode=0o700)
                    native_env = isolated_environment(entry, native, fixture, "cli-local")
                    if backend == "gemini":
                        settings = native / ".gemini/settings.json"
                        payload = json.loads(settings.read_text())
                        payload["security"]["auth"]["selectedType"] = "gemini-api-key"
                        settings.write_text(json.dumps(payload))
                        native_env.update(GEMINI_API_KEY=fixture.key, GEMINI_MODEL=fixture.models[0],
                                          GOOGLE_GEMINI_BASE_URL=fixture.url)
                    spec["env"].update(native_env)
                    env.update(native_env)
                    if backend == "openclaw":
                        owned.enter_context(openclaw_native_gateway(spec, env, native))
                env["ACP_APPROVAL_FIXTURE_KEY"] = fixture.key
            adapter, endpoint = {
                "claude": ("anthropic", fixture.url + "/v1"),
                "codex": ("openai-compatible", fixture.url + "/v1/chat/completions"),
                "gemini": ("google", fixture.url + "/v1beta"),
            }.get(backend, ("openai", fixture.url + "/v1"))
            config = {
                "workspace_dir": str(workspace), "gateway": {"auth_enabled": False},
                "agents": {agent: {"enabled": True, "runtime": {"kind": "acp", "backend": backend},
                                   "defaults": {"skills": {"mode": "fixed", "value": []},
                                                "mcp": {"mode": "fixed", "value": []}}}},
                "acp": {backend: spec},
                "providers": {"fixture": {"name": "Approval fixture", "api": endpoint,
                    "adapter": adapter, "env": ["ACP_APPROVAL_FIXTURE_KEY"],
                    "models": {fixture.models[0]: {"status": "active", "temperature": True,
                        "tool_call": True, "limit": {"context": 100000,
                            "output": 16384 if backend == "gemini" else 1024}}}}},
            }
            with gateway(binary, root, env) as url:
                request(url, "/global/config", config, method="PATCH")
                query = "?" + urlencode({"directory": str(workspace)})
                other_workspace = root / "other-workspace"
                other_workspace.mkdir()
                other_query = "?" + urlencode({"directory": str(other_workspace)})
                session = json.loads(request(url, "/session" + query, {"title": "ACP approval"}))["id"]
                row["phases"] = phases = []
                for phase, full in PHASES:
                    for file in (workspace / "note.txt", workspace / "terminal.txt", outside):
                        file.unlink(missing_ok=True)
                    (workspace / "keep.txt").write_text("keep")
                    if backend == "gemini":
                        # Gemini's edit preparation reads the file before writing. Use an
                        # existing file for this native path; the host peer tests creation.
                        (workspace / "note.txt").write_text("before Gemini edit")
                    body = {"session_id": session, "agent": agent, "acp_model_source": source,
                            "options": {"full_access": full, "cwd": str(workspace),
                                        "acp_close_client_after_request": entry is None}}
                    if source == "session_model":
                        body["model"] = "fixture/" + fixture.models[0]
                    if entry is None:
                        body["options"]["acp_non_interactive_permissions"] = "deny"
                    # Some suppliers end the current prompt when a user rejects a tool.
                    # Give each restricted probe its own turn to still cover later tools.
                    steps = (None,) if entry is None or backend in ("claude", "codex", "opencode") or full else range(5)
                    decisions, actions, results, advertised = [], [], {}, set()
                    completed = True
                    failures = []
                    for step in steps:
                        turn = phase if step is None else f"{phase}_{step}"
                        fixture.reset(turn, step)
                        body["messages"] = [{"role": "user", "content": "Run approval fixture " + turn}]
                        if entry is None:
                            stream = request(url, "/chat/stream" + query, body)
                            turn_decisions = []
                        else:
                            gemini_write_expected = None
                            if backend == "gemini" and step in (0, 1):
                                gemini_write_expected = lambda: fixture.workspace_edit_issued
                            stream, turn_decisions = interactive_turn(
                                url, query, other_query, body, workspace, phase == "approval",
                                gemini_write_expected=gemini_write_expected,
                            )
                        turn_completed, answer = stream_evidence(stream)
                        if not turn_completed and not any(d["reply"] == "reject" for d in turn_decisions):
                            completed = False
                            failures.extend(stream_failure_categories(stream))
                        decisions.extend(turn_decisions)
                        actions.extend(fixture.actions)
                        results.update(fixture.results)
                        advertised.update(fixture.tool_names)
                    note = workspace / "note.txt"
                    observed = {
                        "phase": phase,
                        "turn_completed": completed,
                        "second_edit": note.is_file() and note.read_text() == CONTENT[-1],
                        "outside_write": outside.is_file(),
                        "terminal_marker": (workspace / "terminal.txt").is_file(),
                        "deletion": not (workspace / "keep.txt").is_file(),
                    }
                    if entry is None:
                        decision = "once" if full else "deny"
                        lifecycle = "new" if phase == "approval" else "load"
                        marker = answer.partition(DONE + " ")[2].strip()
                        observed["peer_observation"] = json.loads(marker) if marker else None
                        observed["permission_outcomes"] = observed["peer_observation"] == [
                            [decision] * 3, full, lifecycle
                        ]
                    else:
                        observed["approvals"] = decisions
                        observed["model_tool_requests"] = actions
                        observed["model_tool_results"] = dict(sorted(results.items()))
                        observed["advertised_tools"] = sorted(advertised)
                        if failures:
                            observed["gateway_failures"] = sorted(set(failures))
                        controls = json.loads(request(url, f"/session/{session}/acp-options" + query))
                        observed["native_mode"] = next((
                            option["current"] for option in controls["options"]
                            if option["kind"] in ({"kind": "mode"}, {"kind": "config", "id": "mode"})
                        ), None)
                        observed["access_status"] = controls.get("access_status")
                    phases.append(observed)
                if entry is None:
                    require(all(p["turn_completed"] and p["second_edit"] and p["permission_outcomes"]
                                and p["outside_write"] == full
                                for p, (_, full) in zip(phases, PHASES)),
                            "desktop fixture did not satisfy the approval contract")
                    row["parallel"] = parallel_host_sessions(url, agent, (workspace, other_workspace))
                    row["status"] = "passed"
                elif backend == "claude":
                    restricted, full, revoked = phases
                    require(all(p["turn_completed"] and len(p["model_tool_results"]) == 5 for p in phases),
                            "real adapter did not complete each tool round")
                    require(restricted["second_edit"] and not revoked["second_edit"],
                            "workspace edits did not follow the current one-time decisions")
                    require([d["reply"] for d in restricted["approvals"]] == ["once"] * 2 + ["reject"] * 3
                            and [d["reply"] for d in revoked["approvals"]] == ["reject"] * 5,
                            "unexpected supplier approval events")
                    for observed, (_, granted) in zip(phases, PHASES):
                        require(all(observed[key] == granted
                                    for key in ("outside_write", "terminal_marker", "deletion")),
                                "sensitive tool effects do not match the current grant")
                    require(full["second_edit"] and not full["approvals"],
                            "native full access did not complete its tools")
                    if source == "cli_local":
                        require([p["native_mode"] for p in phases] == [
                            "default", "bypassPermissions", "default"
                        ], "Claude mode readback did not match the grant")
                    row["status"] = "passed"
                    row["fallback"] = "native edits require one-time confirmation"
                else:
                    verify_native_phases(backend, source, row, phases)


def verify_native_phases(backend, source, row, phases):
    restricted, full, revoked = phases
    if backend == "opencode":
        require(all(not phase["turn_completed"] and not phase["model_tool_requests"]
                    and "opencode_requires_full_access" in phase.get("gateway_failures", [])
                    for phase in (restricted, revoked)),
                "OpenCode approval and revocation must reject native execution before model calls")
        require(not any(phase[key] for phase in (restricted, revoked)
                        for key in ("second_edit", "outside_write", "terminal_marker", "deletion")),
                "OpenCode had native side effects without full access")
        require(full["turn_completed"] and len(full["model_tool_results"]) == 5 and all(
            full[key] for key in ("second_edit", "outside_write", "terminal_marker", "deletion")
        ), "OpenCode full access did not complete its operations")
        row.update(status="passed", fallback="native execution requires a current full-access grant")
        return
    require(all(p["turn_completed"] and len(p["model_tool_requests"]) == 5 for p in phases),
            "real adapter did not execute all five tool scenarios")
    require(not any(p[key] for p in (restricted, revoked)
                    for key in ("outside_write", "terminal_marker", "deletion")),
            "sensitive native operation escaped approval")
    require(len(full["model_tool_results"]) == 5 and full["second_edit"]
            and full["terminal_marker"] and full["deletion"],
            "full access did not complete workspace edits and shell operations")
    if backend != "gemini":
        require(full["outside_write"], "the native outside write did not complete with full access")
    if backend == "codex":
        require(not restricted["second_edit"] and not revoked["second_edit"],
                "Codex read-only allowed a native write")
        require(all(len(p["model_tool_results"]) == 5 for p in (restricted, revoked)),
                "Codex read-only commands did not return results")
        if source == "cli_local":
            require([p["native_mode"] for p in phases] == [
                "read-only", "agent-full-access", "read-only"
            ], "Codex mode readback did not match the grant")
        row["fallback"] = "native read-only; structured host writes tested separately"
    else:
        require(restricted["second_edit"], "restricted workspace edits did not complete")
        require(any(p["approvals"] for p in (restricted, revoked)),
                "no supplier approval event was observed")
        if any(d["workspace_edit"] for d in revoked["approvals"]):
            require(not revoked["second_edit"], "a rejected workspace edit was executed")
        row["fallback"] = "transient host approvals for requested native operations"
        if backend == "gemini":
            if source == "cli_local":
                require(all(p["native_mode"] == "default" for p in phases),
                        "Gemini changed native mode instead of using client approvals")
            row["fallback"] = "client approvals; Gemini keeps native mode and file boundaries"
    row["status"] = "passed"


def verify(binary, report, entries, selected):
    with tempfile.TemporaryDirectory(prefix="shiju-acp-approval-registry-") as name:
        root = Path(name).resolve()
        with gateway(binary, root, environment(root)) as url:
            production = set(json.loads(request(url, "/global/config/acp/defaults")))
            model_sources = {
                key: json.loads(request(url, f"/acp-model-source/{key}")) for key in sorted(production)
            }
    report["registry"] = sorted(production)
    require(production == set(entries), "version manifest differs from production registry")
    require(not selected or set(selected) <= production, "unknown backend selected")
    runnable = []
    for key in sorted(set(selected or ()) or production):
        entry = entries[key]
        for source in ("cli_local", "session_model"):
            row = {"agent": key, "source": source, "version_pin": entry["distribution"]["release"],
                   "status": "blocked", "fallback": "host ACP callbacks only"}
            report["suppliers"].append(row)
            if source == "session_model" and not model_sources[key].get("verified_cli_release"):
                row.update(status="unsupported", reason="production has no verified client model source")
                continue
            runnable.append((key, source, row, entry))

    def run_supplier(item):
        key, source, row, entry = item
        try:
            session_turns(binary, key, source, row, entry)
        except (Unavailable, URLError, TimeoutError, OSError, subprocess.TimeoutExpired) as error:
            row["reason"] = str(error)
        except AssertionError as error:
            row.update(status="failed", reason=str(error))
        print(f"{key} {source}: {row['status']}", file=sys.stderr, flush=True)

    with ThreadPoolExecutor(max_workers=3) as pool:
        list(pool.map(run_supplier, runnable))
    for backend in ("approval-custom", "codex") if not selected else ():
        row = {"agent": backend, "source": "cli_local", "kind": "controlled ACP peer",
               "version": "1", "status": "failed"}
        report["host_fixtures"].append(row)
        try:
            session_turns(binary, backend, "cli_local", row)
        except Unavailable as error:
            if backend == "approval-custom" and "unknown agent backend: approval-custom" in str(error):
                row["status"] = "rejected_by_gateway"
            row["reason"] = str(error)
        except (URLError, TimeoutError, AssertionError) as error:
            row["reason"] = str(error)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gateway", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--agent", action="append", help="run a focused supplier probe")
    args = parser.parse_args()
    report = {"platform": platform.system() + "-" + platform.machine(), "suppliers": [],
              "host_fixtures": []}
    try:
        slot = Path(cargo_slot(os.environ))
        report["cargo_target_dir"] = str(slot)
        require(args.gateway.resolve() == slot / "debug/shiju", "gateway must come from the owned Cargo slot")
        require(args.gateway.is_file(), "build the gateway before running the approval matrix")
        entries = {entry["key"]: entry for entry in json.loads(MANIFEST.read_text())["entries"]}
        report["selected_agents"] = args.agent
        verify(args.gateway.resolve(), report, entries, args.agent)
    except (AssertionError, Unavailable, URLError, TimeoutError) as error:
        report["error"] = str(error)
    report["passed"] = (
        not args.agent and not report.get("error") and bool(report["suppliers"])
        and all(row["status"] in ("passed", "unsupported") for row in report["suppliers"])
        and all(row["status"] in ("passed", "rejected_by_gateway") for row in report["host_fixtures"])
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    return 0 if report["passed"] else 1


def interrupt(_signum, _frame):
    raise KeyboardInterrupt


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--peer":
        peer(Path(sys.argv[2]), Path(sys.argv[3]))
    else:
        signal.signal(signal.SIGTERM, interrupt)
        sys.exit(main())
