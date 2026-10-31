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
        text = f"ACK_{body['model']};USERS={len(users)};SLOW={slow}"
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()
        try:
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
            if frame.get("id") == request_id:
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
    config = {"workspace_dir": str(cwd), "reliability": {"provider_retries": 0}, "providers": {
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
        return {"model_switch": True, "multi_turn": True, "cancel": True, "restart_restore": True,
                "cancelled_history": True}
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
    run(["sessions", "close", "integration"])
    return {"named_controls": True, "model_flag": True, "idle_queue_model_switch": True,
            "active_turn_model_switch": True, "queue_cancel": True, "cancel_recovery": True, "close_by_name": True}


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
        results["client"] = cli_scenarios(acp, shiju, output, env, cwd)
        (output / "results.json").write_text(json.dumps(results, indent=2))
        print(json.dumps(results), flush=True)
    finally:
        cleanup_owners(output, acp)
        server.shutdown()
        server.server_close()


if __name__ == "__main__":
    main()
