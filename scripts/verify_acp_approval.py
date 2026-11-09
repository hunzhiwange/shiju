#!/usr/bin/env python3
"""Verify ACP approval through the desktop chat HTTP route in an isolated gateway."""
import argparse
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
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

from acp_skill_mount_claude import messages_wire
from verify_acp_model_source import ROOT, Unavailable, cargo_slot, isolated_environment, locate

MANIFEST = ROOT / "docs/shiju/acp-model-source/cli-versions.json"
CONTENT = ("first edit", "second edit")
DONE = "ACP_APPROVAL_FIXTURE_DONE"
PHASES = (("approval", False), ("full_access", True), ("revoked", False))
HTTP = build_opener(ProxyHandler({}))


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
    for line in sys.stdin:
        frame = json.loads(line)
        if "id" not in frame:
            continue
        method = frame["method"]
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


@contextmanager
def model_fixture(workspace, outside):
    """Claude Messages fixture. Only model replies are synthetic."""
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_POST(self):
            length = int(self.headers.get("Content-Length", "0"))
            if not 0 < length < 2_000_000 or fixture.key not in (
                self.headers.get("x-api-key"), self.headers.get("Authorization", "").removeprefix("Bearer ")
            ):
                return self.send_error(400)
            request = json.loads(self.rfile.read(length))
            route = self.path.split("?", 1)[0]
            if route == "/v1/messages/count_tokens":
                data, content_type = b'{"input_tokens":1}', "application/json"
            elif route == "/v1/messages":
                message = {"content": DONE}
                with fixture.lock:
                    for turn in request.get("messages", []):
                        content = turn.get("content")
                        if not isinstance(content, list):
                            continue
                        for block in content:
                            if block.get("type") == "tool_result" and block.get("tool_use_id") in fixture.issued:
                                fixture.results[block["tool_use_id"]] = block.get("is_error", False)
                    index = len(fixture.actions)
                    action = next(fixture.calls, None)
                    if action:
                        name, args = action
                        fixture.actions.append(name)
                        tool_id = f"toolu_approval_{fixture.phase}_{index}"
                        fixture.issued.add(tool_id)
                        message = {"tool_calls": [{"id": tool_id, "function": {
                            "name": name, "arguments": json.dumps(args)
                        }}]}
                result, wire = messages_wire(message, request["model"])
                data, content_type = (wire, "text/event-stream") if request.get("stream") else (
                    json.dumps(result).encode(), "application/json"
                )
            else:
                return self.send_error(404)
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

    class Fixture(ThreadingHTTPServer):
        daemon_threads = True
        key = "offline-acp-approval-key"
        models = ("acp-fixture-primary",)
        lock = threading.Lock()

        def reset(self, phase):
            self.phase = phase
            self.actions = []
            self.issued = set()
            self.results = {}
            self.calls = iter([
                *(("Write", {"file_path": str(workspace / "note.txt"), "content": content})
                  for content in CONTENT),
                ("Write", {"file_path": str(outside), "content": "outside edit"}),
                ("Bash", {"command": "touch terminal.txt", "description": "Create a marker"}),
                ("Bash", {"command": "rm keep.txt", "description": "Delete a marker"}),
            ])

    with Fixture(("127.0.0.1", 0), Handler) as fixture:
        fixture.url = f"http://127.0.0.1:{fixture.server_port}"
        thread = threading.Thread(target=fixture.serve_forever)
        thread.start()
        try:
            yield fixture
        finally:
            fixture.shutdown()
            thread.join()


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
        raise Unavailable(f"{path.split('?')[0]} HTTP {error.code}: {payload}") from error


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


def session_turns(binary, backend, source, row, entry=None):
    with tempfile.TemporaryDirectory(prefix="shiju-acp-approval-") as name:
        root = Path(name).resolve()
        env = environment(root)
        workspace = root / "workspace"
        workspace.mkdir()
        outside = root / "outside.txt"
        agent = "approval-fixture"
        with model_fixture(workspace, outside) as fixture:
            if entry is None:
                spec = {"command": sys.executable, "args": [
                    str(Path(__file__).resolve()), "--peer", str(workspace), str(outside)
                ], "env": {}}
            else:
                spec = locate(entry, env)
                if source == "cli_local":
                    native = root / "native"
                    native.mkdir(mode=0o700)
                    env.update(isolated_environment(entry, native, fixture, "cli-local"))
                env["ACP_APPROVAL_FIXTURE_KEY"] = fixture.key
            config = {
                "workspace_dir": str(workspace), "gateway": {"auth_enabled": False},
                "agents": {agent: {"enabled": True, "runtime": {"kind": "acp", "backend": backend},
                                   "defaults": {"skills": {"mode": "fixed", "value": []},
                                                "mcp": {"mode": "fixed", "value": []}}}},
                "acp": {backend: spec},
                "providers": {"fixture": {"name": "Approval fixture", "api": fixture.url + "/v1",
                    "adapter": "anthropic", "env": ["ACP_APPROVAL_FIXTURE_KEY"],
                    "models": {fixture.models[0]: {"status": "active", "temperature": True,
                        "tool_call": True, "limit": {"context": 100000, "output": 1024}}}}},
            }
            with gateway(binary, root, env) as url:
                request(url, "/global/config", config, method="PATCH")
                query = "?" + urlencode({"directory": str(workspace)})
                session = json.loads(request(url, "/session" + query, {"title": "ACP approval"}))["id"]
                row["phases"] = phases = []
                for phase, full in PHASES:
                    for file in (workspace / "note.txt", workspace / "terminal.txt", outside):
                        file.unlink(missing_ok=True)
                    (workspace / "keep.txt").write_text("keep")
                    fixture.reset(phase)
                    body = {"session_id": session, "agent": agent, "acp_model_source": source,
                            "messages": [{"role": "user", "content": "Run approval fixture " + phase}],
                            "options": {"full_access": full, "cwd": str(workspace),
                                        "acp_non_interactive_permissions": "deny",
                                        "acp_close_client_after_request": entry is None}}
                    if source == "session_model":
                        body["model"] = "fixture/" + fixture.models[0]
                    stream = request(url, "/chat/stream" + query, body)
                    events = [json.loads(line.removeprefix("data: "))
                              for line in stream.splitlines() if line.startswith("data: {")]
                    types = {event.get("type") for event in events}
                    answer = "".join(event["delta"] for event in events if event.get("type") == "chat.delta")
                    note = workspace / "note.txt"
                    observed = {
                        "phase": phase,
                        "turn_completed": DONE in answer and "chat.done" in types and "chat.error" not in types,
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
                        observed["model_tool_requests"] = list(fixture.actions)
                        observed["model_tool_results"] = dict(sorted(fixture.results.items()))
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
                    row["status"] = "passed"
                else:
                    restricted = (p for p in phases if p["phase"] != "full_access")
                    if any(not phase["second_edit"] for phase in restricted):
                        row["status"] = "failed"
                        row["reason"] = "native workspace edits did not complete in approval mode"
                    else:
                        row["status"] = "incomplete"
                        row["reason"] = "supplier approval-event and replay evidence missing"


def verify(binary, report, entries):
    with tempfile.TemporaryDirectory(prefix="shiju-acp-approval-registry-") as name:
        root = Path(name).resolve()
        with gateway(binary, root, environment(root)) as url:
            production = set(json.loads(request(url, "/global/config/acp/defaults")))
    report["registry"] = sorted(production)
    require(production == set(entries), "version manifest differs from production registry")
    for key in sorted(production):
        entry = entries[key]
        for source in ("cli_local", "session_model"):
            row = {"agent": key, "source": source, "version_pin": entry["distribution"]["release"],
                   "status": "blocked", "fallback": "host ACP callbacks only"}
            report["suppliers"].append(row)
            recipes = entry.get("offline_acceptance", {})
            if source.replace("_", "-") not in recipes:
                row["reason"] = "no reviewed offline model configuration for this source"
            elif key != "claude":
                row["reason"] = "no deterministic approval tool sequence for this adapter"
            else:
                try:
                    session_turns(binary, key, source, row, entry)
                except (Unavailable, URLError, TimeoutError, AssertionError) as error:
                    row["reason"] = str(error)
    for backend in ("approval-custom", "codex"):
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
    args = parser.parse_args()
    report = {"platform": platform.system() + "-" + platform.machine(), "suppliers": [],
              "host_fixtures": []}
    try:
        slot = Path(cargo_slot(os.environ))
        report["cargo_target_dir"] = str(slot)
        require(args.gateway.resolve() == slot / "debug/shiju", "gateway must come from the owned Cargo slot")
        require(args.gateway.is_file(), "build the gateway before running the approval matrix")
        entries = {entry["key"]: entry for entry in json.loads(MANIFEST.read_text())["entries"]}
        verify(args.gateway.resolve(), report, entries)
    except (AssertionError, Unavailable, URLError, TimeoutError) as error:
        report["error"] = str(error)
    report["passed"] = (
        not report.get("error") and bool(report["suppliers"])
        and all(row["status"] == "passed" for row in report["suppliers"])
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
