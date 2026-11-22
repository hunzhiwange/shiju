#!/usr/bin/env python3
"""Exercise built ACP client/server binaries against an isolated streaming provider."""
import argparse
import json
import os
from pathlib import Path
import platform
import queue
import signal
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time
import uuid
from contextlib import ExitStack, contextmanager
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
    def __init__(self, command, cwd, env, log, *, own_session=False):
        self.log = log.open("w")
        self.child = subprocess.Popen(command, cwd=cwd, env=env, stdin=subprocess.PIPE,
                                      stdout=subprocess.PIPE, stderr=self.log, text=True, bufsize=1,
                                      start_new_session=own_session)
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
            remaining = deadline - time.monotonic()
            require(remaining > 0, f"ACP response timed out for {method}")
            frame = self.frames.get(timeout=remaining)
            require("eof" not in frame and "invalid_stdout" not in frame, str(frame))
            if frame.get("id") == request_id and "method" not in frame:
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


def prepare(output, port, config_overrides=None):
    from system_config_fixture import seed_system_config

    home, cwd = output / "home", output / "cwd"
    home.mkdir()
    cwd.mkdir()
    env = {key: value for key, value in os.environ.items()
           if key in ("PATH", "TMPDIR", "LANG", "LC_ALL", "SHELL", "USER", "LOGNAME", "SYSTEMROOT")}
    env.update(HOME=str(home), VIBEWINDOW_CONFIG_DIR=str(home / ".shiju-harness-dev"),
               ACP_FIXTURE_KEY="fixture-key", RUST_LOG="warn")
    config = {"workspace_dir": str(cwd), "reliability": {"provider_retries": 0}, "providers": {
        "fixture": {"name": "Fixture", "api": f"http://127.0.0.1:{port}/v1", "adapter": "openai",
                    "env": ["ACP_FIXTURE_KEY"], "models": {
                        model: {"status": "active", "tool_call": True,
                                "limit": {"context": 100000, "output": 1024}}
                        for model in ("alpha", "beta")}}}}
    config.update(config_overrides or {})
    database = home / ".shiju-harness-dev/database/vibe-window.sqlite3"
    database.parent.mkdir(parents=True)
    with sqlite3.connect(database) as connection:
        seed_system_config(connection, config)
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
    for path in (output / "home/.shiju-harness-dev/acp/queues").glob("*.lock"):
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


NATIVE_MCP = "mcp__native_probe__read_probe"
ATTACHED_MCP = "mcp__attached_probe__read_probe"


def platform_receipts(messages, marker, issued):
    anchors = [index for index, message in enumerate(messages)
               if message["role"] == "user" and marker in json.dumps(message["content"])]
    require(len(anchors) == 1, "model continuation lacks a unique current prompt")
    current = messages[anchors[0] + 1:]
    calls = [call["id"] for message in current if message["role"] == "assistant"
             for call in message.get("tool_calls") or []]
    receipts = [message for message in current if message["role"] == "tool"]
    require(calls == issued == [message["tool_call_id"] for message in receipts],
            "model continuation has missing, foreign or duplicate call IDs")
    return receipts


def verify_platform_catalog(tools, *, mounted):
    require(NATIVE_MCP in tools, "native MCP missing")
    require((ATTACHED_MCP in tools) == mounted,
            "attached MCP missing" if mounted else "removed MCP still advertised")


def verify_platform_audit(before, after, *, mounted, rejected=False):
    require(after[0] == before[0] + 1, "native MCP audit must record one call")
    require(after[1] == before[1] + int(mounted and not rejected),
            "attached MCP audit differs from the current approval and mount state")


@contextmanager
def loopback(handler, **state):
    with ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        server.daemon_threads = True
        vars(server).update(state)
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        try:
            yield server
        finally:
            server.shutdown()
            worker.join()


class PlatformModel(ProviderHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        turn = self.server.turn
        try:
            require(self.path == "/v1/chat/completions" and body.get("stream") is True,
                    "expected streaming OpenAI Chat Completions")
            require(body["model"] == turn["model"], "unexpected requested model")
            messages = body["messages"]
            turn["receipts"] = receipts = platform_receipts(messages, turn["marker"], turn["issued"])
            if turn.get("history"):
                require(any(message["role"] == "user" and turn["history"] in json.dumps(message["content"])
                            for message in messages), "restored history missing from model request")
            tools = {tool["function"]["name"] for tool in body.get("tools", [])}
            step = len(receipts)
            plan = turn["plan"]
            if step:
                name, _, expected = plan[step - 1]
                content = str(receipts[-1]["content"])
                if expected is None:
                    require(turn["attached_value"] not in content
                            and any(word in content.lower() for word in ("denied", "rejected", "拒绝")),
                            "attached MCP refusal lacked its correlated rejection receipt")
                else:
                    require(expected in content, f"{name} continuation missing its fixture value")
            # MCP tools are dynamic. Check the catalog after the real tool_search call.
            if step and plan[step - 1][0] == "tool_search":
                catalog = str(receipts[-1]["content"])
                candidates = {name for name in (NATIVE_MCP, ATTACHED_MCP)
                              if name in tools or name in catalog}
                verify_platform_catalog(candidates, mounted=turn["mounted"])
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            if turn.get("cancel"):
                for _ in range(200):
                    self.chunk(body["model"], {"content": "PLATFORM_WAITING"}, None)
                    time.sleep(.1)
                raise AssertionError("cancelled prompt kept its model stream open")
            elif step == len(plan):
                self.chunk(body["model"], {"content": "PLATFORM_COMPLETE"}, None)
                self.chunk(body["model"], {}, "stop")
            else:
                name, args, _ = plan[step]
                require(name in tools, f"tool was not offered by the runtime: {name}")
                call_id = "platform_" + uuid.uuid4().hex
                turn["issued"].append(call_id)
                self.chunk(body["model"], {"tool_calls": [{
                    "index": 0, "id": call_id, "type": "function",
                    "function": {"name": name, "arguments": json.dumps(args)},
                }]}, None)
                self.chunk(body["model"], {}, "tool_calls")
            self.wfile.write(b"data: [DONE]\n\n")
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            if not turn.get("cancel"):
                turn["failure"] = "model connection closed before completing the turn"
        except Exception as error:
            turn["failure"] = str(error)
            raise


def owned_session_pids(sid):
    listing = subprocess.check_output(["ps", "-axo", "pid=,sid=,stat="], text=True)
    return {int(pid) for pid, session, status in map(str.split, listing.splitlines())
            if int(session) == sid and not status.startswith("Z")}


@contextmanager
def platform_process(command, cwd, env, log):
    process = Process(command, cwd, env, log, own_session=True)
    sid = process.child.pid
    try:
        yield process
    finally:
        try:
            process.close()
            deadline = time.monotonic() + 5
            while owned_session_pids(sid) and time.monotonic() < deadline:
                time.sleep(.05)
            require(process.child.returncode == 0, "ACP host did not exit cleanly")
            require(not owned_session_pids(sid), "ACP host leaked processes in its own OS session")
        finally:
            for pid in owned_session_pids(sid):
                try:
                    if os.getsid(pid) == sid:
                        os.kill(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass


def platform_scenarios(shiju, output):
    from acp_mount_integration import McpHandler
    from verify_acp_skill_coexistence import make_skill

    with ExitStack() as stack:
        model = stack.enter_context(loopback(PlatformModel))
        configuration = {"autonomy": {"level": "supervised", "always_ask": [ATTACHED_MCP]}}
        env, cwd = prepare(output, model.server_port, configuration)
        native = make_skill(cwd / ".shiju/skills", "native")
        attached = make_skill(output / "skills", "attached")
        database = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "database/vibe-window.sqlite3"
        with sqlite3.connect(database) as connection:
            connection.execute(
                "CREATE TABLE gateway_settings (platform_id TEXT NOT NULL, company_id TEXT NOT NULL, "
                "namespace TEXT NOT NULL, config_key TEXT NOT NULL, payload_json TEXT NOT NULL, "
                "updated_at_ms INTEGER NOT NULL, PRIMARY KEY(platform_id,company_id,namespace,config_key))"
            )
            preferences = {"permission": {"skill": {native["name"]: "allow", attached["name"]: "allow"}}}
            connection.execute("INSERT INTO gateway_settings VALUES (?,?,?,?,?,?)", (
                "10000", "100000000", "desktop.preferences", "default", json.dumps(preferences), 1,
            ))
        configs, audits, values = {}, [], []
        for name in ("native_probe", "attached_probe"):
            attachment = output / (name + ".txt")
            value = uuid.uuid4().hex
            attachment.write_text(value)
            audit = output / (name + ".audit")
            if name == "native_probe":
                configs[name] = {"name": name, "command": sys.executable, "args": [
                    str(ROOT / "scripts/acp_mount_integration.py"), "--mcp-stdio",
                    str(attachment), str(audit),
                ]}
            else:
                mcp = stack.enter_context(loopback(
                    McpHandler, attachment=attachment, audit=audit,
                    channels={}, stopping=threading.Event(),
                ))
                configs[name] = {"name": name, "type": "http",
                                 "url": f"http://127.0.0.1:{mcp.server_port}/mcp"}
            audits.append(audit)
            values.append(value)
        (cwd / ".vwacprc.json").write_text(json.dumps({"mcpServers": [configs["native_probe"]]}))

        def audit_counts():
            return tuple(len(path.read_text().splitlines()) if path.exists() else 0 for path in audits)

        def connect(log):
            return platform_process([str(shiju), "acp"], cwd, env, output / log)

        def initialize(client):
            capabilities, _ = client.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
            require(capabilities["protocolVersion"] == 1, "ACP protocol version")
            require(capabilities["_meta"]["shiju.mounts"]["setMounts"], "ACP mount update capability")
            return capabilities["agentCapabilities"]

        def probe(client, session, phase, mounted=False, rejected=False, history=None):
            marker = f"ACP_{phase}_" + uuid.uuid4().hex
            plan = [
                ("skill", {"skill": native["name"]}, f'Skill "{native["name"]}" loaded.'),
                ("file_read", {"path": str(native["path"] / "references/value.txt")}, native["attachment"]),
                ("skill", {"skill": attached["name"]},
                 f'Skill "{attached["name"]}" loaded.' if mounted else "未找到技能"),
            ]
            if mounted:
                plan.append(("file_read", {"path": str(attached["path"] / "references/value.txt")},
                             attached["attachment"]))
            plan += [
                ("tool_search", {"query": "read_probe", "limit": 10}, NATIVE_MCP),
                (NATIVE_MCP, {}, values[0]),
            ]
            if mounted:
                plan.append((ATTACHED_MCP, {}, None if rejected else values[1]))
            model.turn = {"marker": marker, "model": "beta", "plan": plan, "issued": [],
                          "mounted": mounted, "history": history, "attached_value": values[1]}
            before = audit_counts()
            decisions = []

            def permission(frame):
                if frame.get("method") != "session/request_permission":
                    return
                params = frame["params"]
                require(params["sessionId"] == session, "permission requested for another session")
                step = len(model.turn["issued"])
                require(step and step <= len(plan), "permission outside a model tool call")
                tool = plan[step - 1][0]
                call = params["toolCall"]
                require(call["toolCallId"] == model.turn["issued"][-1] and call["title"] == tool,
                        "permission did not identify the current model call")
                denied = tool == ATTACHED_MCP and rejected
                kind = "reject_once" if denied else "allow_once"
                if tool == ATTACHED_MCP:
                    require(audit_counts()[1] == before[1], "attached MCP executed before approval")
                    decisions.append(kind)
                option = next(option for option in params["options"] if option["kind"] == kind)
                response = {"jsonrpc": "2.0", "id": frame["id"], "result": {
                    "outcome": {"outcome": "selected", "optionId": option["optionId"]}}}
                client.child.stdin.write(json.dumps(response) + "\n")
                client.child.stdin.flush()

            try:
                result, events = client.call("session/prompt", {"sessionId": session, "prompt": [
                    {"type": "text", "text": marker}]}, on_frame=permission)
            except (AssertionError, queue.Empty):
                require(not model.turn.get("failure"), model.turn.get("failure"))
                raise
            require(not model.turn.get("failure"), model.turn.get("failure"))
            require(all(frame["params"]["sessionId"] == session for frame in events
                        if frame.get("method") == "session/update"), "foreign ACP session update")
            require(result["stopReason"] == "end_turn" and "PLATFORM_COMPLETE" in text_updates(events),
                    f"{phase} did not finish after all correlated receipts")
            require(decisions == (["reject_once" if rejected else "allow_once"] if mounted else []),
                    "unexpected attached MCP approval receipts")
            verify_platform_audit(before, audit_counts(), mounted=mounted, rejected=rejected)
            return marker

        report = {}
        with connect("platform-host.log") as client:
            initialize(client)
            params = {"cwd": str(cwd), "mcpServers": []}
            created, _ = client.call("session/new", params)
            sid = created["sessionId"]
            client.call("session/set_model", {"sessionId": sid, "modelId": "fixture/beta"})
            revision = created["_meta"]["shiju.mounts"]["revision"]
            require(audit_counts() == (0, 0), "discovery executed an MCP tool")
            baseline = probe(client, sid, "baseline")
            report["baseline"] = True

            def update(servers, skills):
                nonlocal revision
                before = audit_counts()
                changed, _ = client.call("_shiju/session/set_mounts", {"sessionId": sid,
                    "expectedRevision": revision, "skills": skills, "mcpServers": servers})
                require(changed["skills"] == [item["name"] for item in skills]
                        and changed["mcpServers"] == [item["name"] for item in servers]
                        and changed["revision"] > revision, "mount revision or readback mismatch")
                require(audit_counts() == before, "mount update executed an MCP tool")
                revision = changed["revision"]

            update([configs["attached_probe"]],
                   [{"name": attached["name"], "path": str(attached["path"])}])
            probe(client, sid, "mounted", mounted=True)
            report["approved"] = True
            probe(client, sid, "rejected", mounted=True, rejected=True)
            report["rejected"] = True
            update([], [])
            probe(client, sid, "removed")
            report["removed"] = True
            marker = "ACP_CANCEL_" + uuid.uuid4().hex
            model.turn = {"marker": marker, "model": "beta", "plan": [], "issued": [], "cancel": True}
            cancelled = None

            def cancel(frame):
                nonlocal cancelled
                if cancelled is None and "PLATFORM_WAITING" in "".join(text_updates([frame])):
                    client.send("session/cancel", {"sessionId": sid}, notification=True)
                    cancelled = time.monotonic()

            before = audit_counts()
            result, _ = client.call("session/prompt", {"sessionId": sid, "prompt": [
                {"type": "text", "text": marker}]}, on_frame=cancel)
            require(cancelled is not None and time.monotonic() - cancelled < 5
                    and result["stopReason"] == "cancelled", "ACP prompt did not cancel promptly")
            require(audit_counts() == before, "cancelled turn called a fixture service")
            report["cancelled"] = True

        with connect("platform-recovered.log") as client:
            agent = initialize(client)
            require(agent.get("loadSession") is True, "ACP load capability")
            before = audit_counts()
            loaded, _ = client.call("session/load", {**params, "sessionId": sid})
            mounts = loaded["_meta"]["shiju.mounts"]
            require(mounts["revision"] == revision and mounts["skills"] == [] and mounts["mcpServers"] == [],
                    "removed mounts reappeared on restart")
            require(loaded["models"]["currentModelId"] == "fixture/beta", "restored model")
            require(audit_counts() == before, "session load executed an MCP tool")
            probe(client, sid, "recovered", history=baseline)
            report["recovered"] = True
            if (agent.get("sessionCapabilities") or {}).get("fork") is not None:
                forked, _ = client.call("session/fork", {**params, "sessionId": sid})
                require(forked["sessionId"] != sid, "fork must bind a different session")
                probe(client, forked["sessionId"], "forked", history=baseline)
                report["fork"] = True
            else:
                report["fork"] = "not advertised"
        report["owned_processes_cleaned"] = True
        return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--shiju", type=Path, default=ROOT / "target-dir/debug/shiju")
    parser.add_argument("--acp", type=Path, default=ROOT / "target-dir/debug/acp")
    parser.add_argument("--output", type=Path)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--mounts", action="store_true", help="Run production skill/MCP mount acceptance")
    modes.add_argument("--linux-platform", action="store_true", help="Run Linux ACP host integration")
    args = parser.parse_args()
    shiju = args.shiju.resolve(strict=True)
    if args.linux_platform:
        require(platform.system() == "Linux" and platform.machine() in ("x86_64", "aarch64")
                and platform.libc_ver()[0] == "glibc" and os.geteuid() != 0,
                "Linux integration requires a non-root GNU x86_64 or aarch64 environment")
    acp = args.acp.resolve(strict=True) if not (args.linux_platform or args.mounts) else None
    output = args.output.resolve() if args.output else Path(tempfile.mkdtemp(prefix="vw-acp-integration-"))
    output.mkdir(parents=True, exist_ok=not args.output)
    if args.linux_platform:
        report = {"architecture": platform.machine(), "passed": False, "checks": {}}
        try:
            report["checks"] = platform_scenarios(shiju, output)
            report["passed"] = True
        finally:
            (output / "platform-results.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report), flush=True)
        return
    if args.mounts:
        from acp_mount_integration import run
        print(json.dumps({"mounts": run(shiju, output)}), flush=True)
        return
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
