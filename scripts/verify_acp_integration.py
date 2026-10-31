#!/usr/bin/env python3
"""Exercise built ACP client/server binaries against an isolated streaming provider."""
import argparse
import json
import os
from pathlib import Path
import queue
import signal
import sqlite3
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[1]


def require(condition, message):
    if not condition:
        raise AssertionError(message)


class Provider(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, output):
        self.requests = []
        self.lock = threading.Lock()
        self.output = output
        super().__init__(("127.0.0.1", 0), ProviderHandler)


class ProviderHandler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_GET(self):
        data = json.dumps({"object": "list", "data": [
            {"id": model, "object": "model"} for model in ("alpha", "beta")
        ]}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        with self.server.lock:
            self.server.requests.append(body)
            (self.server.output / "provider-requests.json").write_text(
                json.dumps(self.server.requests, ensure_ascii=False, indent=2)
            )
        users = [item for item in body.get("messages", []) if item.get("role") == "user"]
        slow = "SLOW_CANCEL" in json.dumps(users[-1:])
        marker = next((name for name in ("TOOL_ALLOW", "TOOL_REJECT", "TOOL_INVALID",
                       "TOOL_CANCELLED", "TOOL_RPC_ERROR", "TOOL_DISCONNECT", "TOOL_CANCEL",
                       "TOOL_READ", "TOOL_FAIL") if name in json.dumps(users[-1:])), None)
        last_user = max((i for i, item in enumerate(body.get("messages", []))
                         if item.get("role") == "user"), default=-1)
        results = [item for item in body.get("messages", [])[last_user + 1:]
                   if item.get("role") == "tool"]
        text = f"ACK_{body['model']};USERS={len(users)};SLOW={slow}"
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()
        try:
            if marker and not results:
                read = marker in ("TOOL_READ", "TOOL_FAIL")
                arguments = ({"path": "fixture.txt" if marker == "TOOL_READ" else "missing.txt"}
                             if read else {"path": f"{marker}.txt", "content": marker})
                tool = "file_read" if read else "file_write"
                self.chunk(body["model"], {"tool_calls": [{"index": 0, "id": f"call_{marker}",
                           "type": "function", "function": {
                               "name": tool,
                               "arguments": json.dumps(arguments)}}]}, None)
                self.chunk(body["model"], {}, "tool_calls")
                self.wfile.write(b"data: [DONE]\n\n")
                self.wfile.flush()
                return
            if results:
                text = "TOOL_RESULT=" + json.dumps(results, ensure_ascii=False)
            for content in ([text] + [" slow"] * 200 if slow else [text]):
                self.chunk(body["model"], {"content": content}, None)
                if slow:
                    time.sleep(0.1)
            self.chunk(body["model"], {}, "stop")
            self.wfile.write(b"data: [DONE]\n\n")
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            pass

    def chunk(self, model, delta, finish):
        frame = {"id": "chatcmpl-fixture", "object": "chat.completion.chunk",
                 "created": 1, "model": model,
                 "choices": [{"index": 0, "delta": delta, "finish_reason": finish}]}
        if finish:
            frame["usage"] = {"prompt_tokens": 100, "completion_tokens": 10, "total_tokens": 110}
        self.wfile.write(("data: " + json.dumps(frame) + "\n\n").encode())
        self.wfile.flush()


class Process:
    def __init__(self, command, cwd, env, log):
        self.transcript = log.with_suffix(".frames.json")
        self.log = log.open("w")
        self.child = subprocess.Popen(command, cwd=cwd, env=env, stdin=subprocess.PIPE,
                                      stdout=subprocess.PIPE, stderr=self.log, text=True, bufsize=1)
        self.frames = queue.Queue()
        self.received = []
        self.reader = threading.Thread(target=self.read, daemon=True)
        self.reader.start()
        self.next_id = 0

    def read(self):
        for line in self.child.stdout:
            try:
                frame = json.loads(line)
            except ValueError:
                frame = {"invalid_stdout": line}
            self.received.append(frame)
            self.frames.put(frame)
        self.frames.put({"eof": True})

    def send(self, method, params, notification=False):
        self.next_id += 1
        frame = {"jsonrpc": "2.0", "method": method, "params": params}
        if not notification:
            frame["id"] = self.next_id
        self.child.stdin.write(json.dumps(frame) + "\n")
        self.child.stdin.flush()
        return self.next_id

    def call(self, method, params, on_frame=None):
        request_id = self.send(method, params)
        events = []
        deadline = time.monotonic() + 30
        while True:
            frame = self.frames.get(timeout=max(0.01, deadline - time.monotonic()))
            require("eof" not in frame and "invalid_stdout" not in frame, str(frame))
            if "method" not in frame and frame.get("id") == request_id:
                require("error" not in frame, str(frame))
                return frame["result"], events
            events.append(frame)
            if on_frame:
                on_frame(frame)

    def close(self):
        if self.child.stdin and not self.child.stdin.closed:
            self.child.stdin.close()
        try:
            self.child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.child.kill()
            self.child.wait()
        self.reader.join(timeout=2)
        self.transcript.write_text(json.dumps(self.received, ensure_ascii=False, indent=2))
        self.log.close()


def text_updates(frames):
    return [frame["params"]["update"]["content"]["text"] for frame in frames
            if frame.get("method") == "session/update"
            and frame["params"]["update"].get("sessionUpdate") == "agent_message_chunk"]


def prepare(output, port):
    home, cwd = output / "home", output / "cwd"
    home.mkdir()
    cwd.mkdir()
    env = {key: value for key, value in os.environ.items()
           if key in ("PATH", "TMPDIR", "LANG", "LC_ALL", "SHELL", "USER", "LOGNAME", "SYSTEMROOT")}
    env.update(HOME=str(home), VIBEWINDOW_CONFIG_DIR=str(home / ".shiju-agent-dev"),
               ACP_FIXTURE_KEY="fixture-key", RUST_LOG="warn")
    config = {"reliability": {"provider_retries": 0},
              "autonomy": {"level": "supervised", "always_ask": ["file_write"]}, "providers": {
        "fixture": {"name": "Fixture", "api": f"http://127.0.0.1:{port}/v1", "adapter": "openai",
                    "env": ["ACP_FIXTURE_KEY"], "models": {
                        model: {"status": "active", "tool_call": True,
                                "limit": {"context": 100000, "output": 1024}}
                        for model in ("alpha", "beta")}}}}
    database = home / ".shiju-agent-dev/database/vibe-window.sqlite3"
    database.parent.mkdir(parents=True)
    with sqlite3.connect(database) as connection:
        connection.execute("CREATE TABLE system_configs (platform_id TEXT, company_id TEXT, "
                           "payload_json TEXT, updated_at_ms INTEGER, PRIMARY KEY(platform_id,company_id))")
        connection.execute("INSERT INTO system_configs VALUES (?,?,?,?)",
                           ("10000", "100000000", json.dumps(config), 1))
    return env, cwd


def tool_scenarios(shiju, output, env, cwd):
    clients = []
    (cwd / "fixture.txt").write_text("ACP_TOOL_FIXTURE")

    def connect():
        client = Process([str(shiju), "acp"], cwd, env, output / f"tools-{len(clients)}.log")
        clients.append(client)
        client.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
        return client

    def prompt(client, sid, marker, on_frame=None):
        return client.call("session/prompt", {"sessionId": sid, "prompt": [
            {"type": "text", "text": marker}]}, on_frame=on_frame)

    try:
        for marker in ("TOOL_ALLOW", "TOOL_REJECT", "TOOL_INVALID", "TOOL_CANCELLED",
                       "TOOL_RPC_ERROR", "TOOL_READ", "TOOL_FAIL", "TOOL_CANCEL",
                       "TOOL_DISCONNECT"):
            client = connect()
            created, _ = client.call("session/new", {"cwd": str(cwd), "mcpServers": []})
            sid = created["sessionId"]
            client.call("session/set_model", {"sessionId": sid, "modelId": "fixture/alpha"})
            approvals = []

            def decide(frame):
                if frame.get("method") != "session/request_permission":
                    return
                approvals.append(frame)
                require(frame["params"]["sessionId"] == sid, "permission session identity")
                require(frame["params"]["toolCall"]["toolCallId"] == f"call_{marker}",
                        "permission tool identity")
                preceding = client.received[:client.received.index(frame)]
                require(any(item.get("params", {}).get("update", {}).get("sessionUpdate") == "tool_call"
                            and item["params"]["update"].get("toolCallId") == f"call_{marker}"
                            for item in preceding), "tool start precedes permission request")
                require(not (cwd / f"{marker}.txt").exists(), "no write before approval")
                if marker == "TOOL_DISCONNECT":
                    client.child.stdin.close()
                    return
                if marker == "TOOL_CANCEL":
                    client.send("session/cancel", {"sessionId": sid}, notification=True)
                    return
                response = {"jsonrpc": "2.0", "id": frame["id"]}
                if marker == "TOOL_RPC_ERROR":
                    response["error"] = {"code": -32603, "message": "fixture unavailable"}
                else:
                    outcome = ({"outcome": "cancelled"} if marker == "TOOL_CANCELLED" else {
                        "outcome": "selected", "optionId": {
                            "TOOL_ALLOW": "allow_once", "TOOL_REJECT": "reject_once",
                            "TOOL_INVALID": "unoffered"}[marker]})
                    response["result"] = {"outcome": outcome}
                client.child.stdin.write(json.dumps(response) + "\n")
                client.child.stdin.flush()

            if marker == "TOOL_DISCONNECT":
                client.send("session/prompt", {"sessionId": sid, "prompt": [
                    {"type": "text", "text": marker}]})
                deadline = time.monotonic() + 15
                while not approvals:
                    frame = client.frames.get(timeout=max(.01, deadline - time.monotonic()))
                    require("eof" not in frame, "permission before disconnect")
                    decide(frame)
                require(client.child.wait(timeout=5) == 0, "EOF exits while awaiting permission")
                client.close()
                client = connect()
                _, history = client.call("session/load", {"sessionId": sid, "cwd": str(cwd), "mcpServers": []})
                require(any(marker in json.dumps(item) for item in history), "disconnected prompt persists")
            else:
                result, events = prompt(client, sid, marker, decide)
                require(result["stopReason"] == ("cancelled" if marker == "TOOL_CANCEL" else "end_turn"),
                        f"{marker} terminates")
                updates = [frame["params"]["update"] for frame in events
                           if frame.get("method") == "session/update"]
                require(any(item.get("sessionUpdate") == "tool_call" and
                            item.get("toolCallId") == f"call_{marker}" for item in updates),
                        f"{marker} tool start")
                if marker != "TOOL_CANCEL":
                    status = "completed" if marker in ("TOOL_ALLOW", "TOOL_READ") else "failed"
                    require(any(item.get("sessionUpdate") == "tool_call_update" and
                                item.get("toolCallId") == f"call_{marker}" and
                                item.get("status") == status for item in updates),
                            f"{marker} terminal tool status: {updates}")
                    require(any("TOOL_RESULT=" in item for item in text_updates(events)), "tool result returns to model")
                if marker == "TOOL_READ":
                    require("ACP_TOOL_FIXTURE" in json.dumps(events), "actual file read output")
            require(len(approvals) == (0 if marker in ("TOOL_READ", "TOOL_FAIL") else 1),
                    f"{marker} permission request count")
            target = cwd / f"{marker}.txt"
            require(target.exists() == (marker == "TOOL_ALLOW"), f"{marker} write side effect")
            if target.exists():
                require(target.read_text() == marker, "approved write content")
            if marker == "TOOL_CANCEL":
                # A delayed UI decision must not revive a cancelled native request.
                client.child.stdin.write(json.dumps({"jsonrpc": "2.0", "id": approvals[0]["id"],
                    "result": {"outcome": {"outcome": "selected", "optionId": "allow_once"}}}) + "\n")
                client.child.stdin.flush()
            result, events = prompt(client, sid, "Continue after tool scenario")
            require(result["stopReason"] == "end_turn" and
                    any("USERS=2" in item for item in text_updates(events)), f"{marker} recovery")
            require(target.exists() == (marker == "TOOL_ALLOW"), f"{marker} remains settled")
            client.close()
        return {"tool_read": True, "tool_failure": True, "allow_once": True, "reject_once": True,
                "invalid_option_denied": True, "cancelled_permission_denied": True,
                "permission_rpc_error_denied": True, "cancel_pending_permission": True,
                "disconnect_pending_permission_restore": True}
    finally:
        for client in clients:
            client.close()


def protocol_scenarios(shiju, output, env, cwd):
    sid = None
    with_client = []
    try:
        def connect():
            client = Process([str(shiju), "acp"], cwd, env, output / f"agent-{len(with_client)}.log")
            with_client.append(client)
            client.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
            return client
        client = connect()
        created, _ = client.call("session/new", {"cwd": str(cwd), "mcpServers": []})
        sid = created["sessionId"]
        require([item["modelId"] for item in created["models"]["availableModels"]]
                == ["auto", "fixture/alpha", "fixture/beta"], "advertised model catalog")
        client.call("session/set_config_option", {"sessionId": sid, "configId": "model", "value": "fixture/alpha"})
        for index, model in enumerate(("alpha", "beta"), 1):
            if index == 2:
                client.call("session/set_model", {"sessionId": sid, "modelId": "fixture/beta"})
            result, events = client.call("session/prompt", {"sessionId": sid, "prompt": [
                {"type": "text", "text": "MARKER_FIRST" if index == 1 else "Follow up"}]})
            require(result["stopReason"] == "end_turn", "completed turn")
            require(f"ACK_{model};USERS={index};SLOW=False" in text_updates(events), "model and history at HTTP provider")
        client.close()
        client = connect()
        loaded, history = client.call("session/load", {"sessionId": sid, "cwd": str(cwd), "mcpServers": []})
        require(loaded["models"]["currentModelId"] == "fixture/beta", "restored model")
        require(len([item for item in history if item.get("method") == "session/update"]) == 4, "two restored turns")
        cancelled = False
        def cancel(frame):
            nonlocal cancelled
            if not cancelled and any("SLOW=True" in item for item in text_updates([frame])):
                client.send("session/cancel", {"sessionId": sid}, notification=True)
                cancelled = True
        started = time.monotonic()
        result, _ = client.call("session/prompt", {"sessionId": sid, "prompt": [
            {"type": "text", "text": "SLOW_CANCEL"}]}, on_frame=cancel)
        require(cancelled and result["stopReason"] == "cancelled", "cancelled protocol turn")
        require(time.monotonic() - started < 5, "prompt cancellation latency")
        client.close()
        client = connect()
        _, history = client.call("session/load", {"sessionId": sid, "cwd": str(cwd), "mcpServers": []})
        require(any("SLOW_CANCEL" in json.dumps(item) for item in history), "cancelled user prompt persists")
        require(any("SLOW=True" in item for item in text_updates(history)), "cancelled partial response persists")
        client.send("session/prompt", {"sessionId": sid, "prompt": [
            {"type": "text", "text": "SLOW_CANCEL DISCONNECT_STREAM"}]})
        deadline = time.monotonic() + 15
        while True:
            frame = client.frames.get(timeout=max(.01, deadline - time.monotonic()))
            require("eof" not in frame, "stream before disconnect")
            if any("SLOW=True" in item for item in text_updates([frame])):
                break
        client.child.stdin.close()
        require(client.child.wait(timeout=5) == 0, "EOF cancels streaming prompt")
        client.close()
        client = connect()
        _, history = client.call("session/load", {"sessionId": sid, "cwd": str(cwd), "mcpServers": []})
        require(any("DISCONNECT_STREAM" in json.dumps(item) for item in history), "disconnected streaming prompt persists")
        require(len([frame for frame in history if frame.get("params", {}).get("update", {}).get(
            "sessionUpdate") == "user_message_chunk"]) == 4, "disconnected turn saved exactly once")
        result, events = client.call("session/prompt", {"sessionId": sid, "prompt": [
            {"type": "text", "text": "Continue after streaming disconnect"}]})
        require(result["stopReason"] == "end_turn" and
                "ACK_beta;USERS=5;SLOW=False" in text_updates(events), "stream disconnect recovery")
        return {"model_switch": True, "multi_turn": True, "cancel": True, "restart_restore": True,
                "cancelled_history": True, "stream_disconnect_restore": True}
    finally:
        for client in with_client:
            client.close()


def cli_scenarios(acp, shiju, output, env, cwd):
    base = [str(acp), "--agent", f"{shiju} acp", "--cwd", str(cwd), "--format", "json", "--timeout", "20", "--ttl", "15"]
    calls = []
    def run(args, success=True):
        result = subprocess.run(base + args, cwd=cwd, env=env, capture_output=True, text=True, timeout=30)
        calls.append({"args": args, "exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr})
        (output / "client-calls.json").write_text(json.dumps(calls, ensure_ascii=False, indent=2))
        require((result.returncode == 0) == success, f"client {args}: {result.stderr}")
        return [json.loads(line) for line in result.stdout.splitlines()]
    created = run(["--model", "fixture/alpha", "sessions", "new", "--name", "integration"])[0]
    sid = created["vwacpRecordId"]
    details = run(["sessions", "show", "integration"])
    require("fixture/alpha" in json.dumps(details) and "fixture/beta" in json.dumps(details), "saved model catalog")
    state = details[0]["session"]["vwacp"]
    require(state["current_model_id"] == "fixture/alpha" and state["config_options"][0]["currentValue"] == "fixture/alpha", "current model and selector agree")
    run(["-s", "integration", "set", "model", "fixture/beta"])
    first = run(["-s", "integration", "prompt", "MARKER_FIRST cli"])
    require("ACK_beta;USERS=1;SLOW=False" in text_updates(first), "named model selection")
    run(["-s", "integration", "set", "model", "fixture/alpha"])
    second = run(["-s", "integration", "--model", "fixture/beta", "prompt", "Follow up"])
    require("ACK_beta;USERS=2;SLOW=False" in text_updates(second), "model flag and multi-turn history")
    run(["-s", "integration", "set", "model", "fixture/missing"], success=False)
    status = run(["-s", "integration", "status"])
    require(sid in json.dumps(status), "named status target")
    slow = Process(base + ["-s", "integration", "prompt", "SLOW_CANCEL"], cwd, env, output / "client-cancel.log")
    try:
        deadline = time.monotonic() + 20
        while True:
            frame = slow.frames.get(timeout=max(.01, deadline - time.monotonic()))
            require("eof" not in frame and "invalid_stdout" not in frame, "streaming client stdout")
            if any("SLOW=True" in item for item in text_updates([frame])):
                break
        run(["-s", "integration", "set", "model", "fixture/alpha"])
        started = time.monotonic()
        result = run(["-s", "integration", "cancel"])
        require(result[0]["cancelled"] is True, "queue cancellation accepted")
        require(slow.child.wait(timeout=5) == 0, "cancelled client exits")
        slow.reader.join(timeout=2)
        require(time.monotonic() - started < 5, "queue cancellation latency")
        require(any(item.get("result", {}).get("stopReason") == "cancelled" for item in slow.received), "cancelled SDK stop reason")
    finally:
        slow.close()
    follow_up = run(["-s", "integration", "prompt", "Resume after cancellation"])
    require("ACK_alpha;USERS=4;SLOW=False" in text_updates(follow_up), "cancelled conversation resumes")
    owners = [json.loads(path.read_text()) for path in
              (output / "home/.shiju-agent-dev/acp/queues").glob("*.lock")]
    owners = [owner for owner in owners if owner.get("sessionId") == sid]
    require(len(owners) == 1, "isolated queue owner exists")
    owner = owners[0]
    command = subprocess.run(["ps", "-p", str(owner["pid"]), "-o", "command="],
                             capture_output=True, text=True, check=True).stdout.strip()
    require(command.startswith(str(acp)), "fault injection targets fixture owner")
    os.kill(owner["pid"], signal.SIGKILL)
    recovered = run(["-s", "integration", "prompt", "Continue after owner crash"])
    require("ACK_alpha;USERS=5;SLOW=False" in text_updates(recovered), "crashed queue owner reconnects")
    run(["sessions", "close", "integration"])
    (cwd / "TOOL_ALLOW.txt").unlink()
    for policy, marker, expected in (("--approve-all", "TOOL_ALLOW", "completed"),
                                     ("--deny-all", "TOOL_REJECT", "failed")):
        name = marker.lower()
        run([policy, "--model", "fixture/alpha", "sessions", "new", "--name", name])
        frames = run([policy, "-s", name, "prompt", marker])
        require(any(frame.get("params", {}).get("update", {}).get("status") == expected
                    for frame in frames), f"client {policy} tool status")
        require((cwd / f"{marker}.txt").exists() == (expected == "completed"),
                f"client {policy} side effect")
        run(["sessions", "close", name])
    return {"named_controls": True, "model_flag": True, "idle_queue_model_switch": True,
            "active_turn_model_switch": True, "queue_cancel": True, "cancel_recovery": True,
            "owner_crash_recovery": True, "close_by_name": True,
            "approve_all_tool": True, "deny_all_tool": True}


def cleanup_owners(output, acp):
    # Leases under this newly created HOME identify only this run's queue owners.
    for path in (output / "home/.shiju-agent-dev/acp/queues").glob("*.lock"):
        try:
            lease = json.loads(path.read_text())
            pid = lease.get("pid")
            if not isinstance(pid, int):
                continue
            command = subprocess.run(["ps", "-p", str(pid), "-o", "command="], capture_output=True, text=True).stdout.strip()
            if command.startswith(str(acp)):
                os.kill(pid, signal.SIGTERM)
        except (OSError, ValueError):
            pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--shiju", type=Path, default=ROOT / "target-dir/debug/shiju")
    parser.add_argument("--acp", type=Path, default=ROOT / "target-dir/debug/acp")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    shiju, acp = args.shiju.resolve(strict=True), args.acp.resolve(strict=True)
    output = args.output.resolve() if args.output else Path(tempfile.mkdtemp(prefix="vw-acp-integration-"))
    output.mkdir(parents=True, exist_ok=not args.output)
    server = Provider(output)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    env, cwd = prepare(output, server.server_port)
    print(f"Evidence: {output}", flush=True)
    try:
        results = {"protocol": protocol_scenarios(shiju, output, env, cwd)}
        results["tools"] = tool_scenarios(shiju, output, env, cwd)
        results["client"] = cli_scenarios(acp, shiju, output, env, cwd)
        (output / "results.json").write_text(json.dumps(results, indent=2))
        print(json.dumps(results), flush=True)
    finally:
        cleanup_owners(output, acp)
        server.shutdown()
        server.server_close()


if __name__ == "__main__":
    main()
