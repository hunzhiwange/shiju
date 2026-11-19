"""Drive the pinned DimCode ACP binary with a loopback model and a read-only MCP server."""
import argparse
from contextlib import contextmanager
import http.server
import json
from pathlib import Path
import queue
import subprocess
import tempfile
import threading
import uuid


NATIVE = "read"
MCP = "coexistence__read_probe"


@contextmanager
def serve(handler):
    with http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        server.daemon_threads = True
        worker = threading.Thread(target=server.serve_forever)
        worker.start()
        try:
            yield server
        finally:
            server.shutdown()
            worker.join()


def verify(binary, transport, root):
    workspace = root / "workspace"
    workspace.mkdir()
    native_file = workspace / "probe.txt"
    attachment = root / "attachment.txt"
    state = root / "v2"
    state.mkdir()
    calls = []
    seen = []
    streams = {}
    mcp_methods = []

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def respond(self, status, body=b"", content_type="application/json"):
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            if not self.path.startswith("/sse/"):
                self.respond(405)
                return
            message_path = self.path.replace("/sse/", "/messages/", 1)
            stream = streams[message_path] = queue.Queue()
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            self.wfile.write(f"event: endpoint\ndata: {message_path}\n\n".encode())
            self.wfile.flush()
            while True:
                try:
                    reply = stream.get(timeout=2)
                except queue.Empty:
                    reply = None
                try:
                    self.wfile.write(b"event: message\ndata: " + reply + b"\n\n" if reply else b": keepalive\n\n")
                    self.wfile.flush()
                except (BrokenPipeError, ConnectionResetError):
                    break

        def do_POST(self):
            frame = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            if self.path.endswith("/chat/completions"):
                self.model(frame)
                return
            if "id" not in frame:
                self.respond(202)
                return
            method = frame["method"]
            mcp_methods.append(method)
            if method == "initialize":
                result = {
                    "protocolVersion": frame["params"]["protocolVersion"],
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "coexistence", "version": "1"},
                }
            elif method == "tools/list":
                result = {"tools": [{
                    "name": "read_probe",
                    "description": "Return the read-only test attachment.",
                    "inputSchema": {"type": "object", "properties": {}},
                }]}
            elif method == "tools/call":
                assert frame["params"]["name"] == "read_probe", frame
                calls.append(frame["id"])
                result = {"content": [{"type": "text", "text": attachment.read_text()}]}
            else:
                self.respond(400)
                return
            reply = json.dumps({"jsonrpc": "2.0", "id": frame["id"], "result": result}).encode()
            if self.path.startswith("/messages/"):
                streams[self.path].put(reply)
                self.respond(202)
            else:
                self.respond(200, reply)

        def model(self, request):
            messages = request["messages"]
            names = {item["function"]["name"] for item in request.get("tools", [])}
            case = [message["content"] for message in messages if message["role"] == "user"][-1]
            results = {
                message["tool_call_id"]: message["content"]
                for message in messages if message["role"] == "tool"
            }
            seen.append((names, results))
            tool_calls = []
            for name, arguments in [(NATIVE, {"path": "probe.txt"}), (MCP, {})]:
                if name == MCP and case not in {"mounted", "cancel", "deny", "loaded", "resumed"}:
                    continue
                if name == NATIVE and case in {"cancel", "deny"}:
                    continue
                if f"call_{case}_{name}" not in results:
                    tool_calls.append({
                        "index": 0, "id": f"call_{case}_{name}", "type": "function",
                        "function": {"name": name, "arguments": json.dumps(arguments)},
                    })
                    break
            if tool_calls:
                chunks = [({"tool_calls": tool_calls}, None), ({}, "tool_calls")]
            else:
                chunks = [({"role": "assistant", "content": "finished"}, "stop")]
            stream = "".join(
                "data: " + json.dumps({
                    "id": "fixture", "object": "chat.completion.chunk",
                    "choices": [{"index": 0, "delta": delta, "finish_reason": reason}],
                }) + "\n\n" for delta, reason in chunks
            ) + "data: [DONE]\n\n"
            self.respond(200, stream.encode(), "text/event-stream")

    with serve(Handler) as server:
        endpoint = f"http://127.0.0.1:{server.server_port}"
        selected = {"providerId": "session-selected", "modelId": "selected-model"}
        (state / "config.json").write_text(json.dumps({
            "defaultProvider": "session-selected", "defaultModel": "selected-model",
            "subagentModel": selected, "assistantModel": selected,
            "metadata": {"subagentDefaultMigratedV1": True},
        }))
        (state / "providers.json").write_text(json.dumps({"providers": [{
            "providerId": "session-selected", "enabled": True, "apiKey": "local-fixture-only",
            "baseUrl": endpoint + "/v1",
            "metadata": {"adapter": "openai-compatible", "activeModelId": "selected-model",
                         "models": [{"id": "selected-model"}]},
        }]}))
        def mount(case):
            return [{"type": transport, "name": "coexistence", "headers": [],
                     "url": f"{endpoint}/{transport}/{case}"}]
        env = {"HOME": str(state), "DIMCODE_HOME": str(state), "PATH": "/usr/bin:/bin"}
        @contextmanager
        def connect():
            with (root / "stderr").open("a") as err, subprocess.Popen(
                [binary, "acp"], cwd=workspace, env=env, text=True,
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=err,
            ) as proc:
                frames = queue.Queue()

                def read_frames():
                    for line in proc.stdout:
                        frames.put(json.loads(line))
                    frames.put(None)

                threading.Thread(target=read_frames, daemon=True).start()

                def send(**frame):
                    proc.stdin.write(json.dumps({"jsonrpc": "2.0", **frame}) + "\n")
                    proc.stdin.flush()

                def rpc(index, method, params, cancel=False):
                    approvals = []
                    updates = []
                    send(id=index, method=method, params=params)
                    while True:
                        frame = frames.get(timeout=25)
                        assert frame is not None, (root / "stderr").read_text()
                        if frame.get("method") == "session/request_permission":
                            request = frame["params"]
                            assert request["sessionId"] == params["sessionId"], request
                            call_id = request["toolCall"]["toolCallId"]
                            assert call_id == f"call_{params['prompt'][0]['text']}_{MCP}", request
                            approvals.append(call_id)
                            option = next(opt for opt in request["options"] if opt["kind"] == "allow_once")
                            outcome = ({"outcome": "cancelled"} if cancel else
                                       {"outcome": "selected", "optionId": option["optionId"]})
                            send(id=frame["id"], result={"outcome": outcome})
                        elif frame.get("method") == "session/update":
                            updates.append(frame["params"]["update"])
                        elif "method" in frame and "id" in frame:
                            send(id=frame["id"], error={"code": -32000, "message": "unexpected host call"})
                        elif frame.get("id") == index:
                            assert "error" not in frame, frame
                            return frame["result"], approvals, updates

                try:
                    info, _, _ = rpc(1, "initialize", {"protocolVersion": 1, "clientCapabilities": {}})
                    assert info["agentInfo"]["version"] == "0.5.28", info
                    capabilities = info["agentCapabilities"]
                    assert capabilities["mcpCapabilities"][transport], capabilities
                    assert capabilities["loadSession"], capabilities
                    yield rpc
                finally:
                    proc.stdin.close()
                    try:
                        proc.wait(timeout=8)
                    except subprocess.TimeoutExpired:
                        proc.terminate()
                        try:
                            proc.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            proc.kill()
                            proc.wait()
                    print(transport, "ACP process exit", proc.returncode, flush=True)
                assert proc.returncode == 0, proc.returncode

        def exercise(rpc, index, case, session_id=None, method="session/new"):
            attached = case in {"mounted", "cancel", "deny", "loaded", "resumed"}
            params = {"cwd": str(workspace), "mcpServers": mount(case) if attached else []}
            observed = len(mcp_methods)
            if session_id is None:
                new, _, _ = rpc(index, method, params)
                session_id = new["sessionId"]
            else:
                rpc(index, method, {**params, "sessionId": session_id})
            permission = "read-only" if case == "deny" else "workspace-write"
            config, _, _ = rpc(index + 1, "session/set_config_option", {
                "sessionId": session_id, "configId": "permission", "value": permission,
            })
            assert any(option["id"] == "permission" and option["currentValue"] == permission
                       for option in config["configOptions"]), config
            native_value = f"native-{uuid.uuid4().hex}"
            mcp_value = f"mcp-{uuid.uuid4().hex}"
            native_file.write_text(native_value)
            attachment.write_text(mcp_value)
            seen.clear()
            count = len(calls)
            result, approvals, updates = rpc(index + 2, "session/prompt", {
                "sessionId": session_id, "prompt": [{"type": "text", "text": case}],
            }, cancel=case == "cancel")
            assert result["stopReason"] == "end_turn", result
            names, results = seen[-1]
            native_id, remote_id = f"call_{case}_{NATIVE}", f"call_{case}_{MCP}"
            native, remote = results.get(native_id, ""), results.get(remote_id, "")
            completed = [(update.get("toolCallId"), update.get("status"))
                         for update in updates if update.get("status") in {"completed", "failed"}]
            methods = mcp_methods[observed:]
            print(transport, case, session_id, permission, result["stopReason"],
                  "native", NATIVE in names, native, "mcp", MCP in names, remote,
                  "approvals", approvals, "updates", completed, "MCP RPC IDs", calls[count:],
                  "MCP methods", methods, flush=True)
            expected_methods = {"initialize", "tools/list"} if attached else set()
            if case in {"mounted", "loaded", "resumed"}:
                expected_methods.add("tools/call")
            assert set(methods) == expected_methods, methods
            assert NATIVE in names and (MCP in names) == attached, case
            assert all(NATIVE in names and (MCP in names) == attached for names, _ in seen if names), case
            if case not in {"cancel", "deny"}:
                assert native_value in native, native
                assert (native_id, "completed") in completed, completed
            if case in {"mounted", "loaded", "resumed"}:
                assert mcp_value in remote and len(calls) == count + 1, remote
                assert approvals == [remote_id], approvals
                assert (remote_id, "completed") in completed, completed
            else:
                assert len(calls) == count
            if case == "cancel":
                assert "Permission request cancelled" in remote and approvals == [remote_id], remote
            if case == "deny":
                assert "Tool call denied" in remote and not approvals, remote
            rpc(index + 3, "session/close", {"sessionId": session_id})
            return session_id

        with connect() as rpc:
            sessions = {
                case: exercise(rpc, index * 4, case)
                for index, case in enumerate(("before", "mounted", "removed", "cancel", "deny"), 1)
            }
            assert len(set(sessions.values())) == len(sessions), sessions
        for method, case in (("session/load", "loaded"), ("session/load", "loaded-withdrawn"),
                             ("session/resume", "resumed"), ("session/resume", "resumed-withdrawn")):
            with connect() as rpc:
                exercise(rpc, 3, case, sessions["mounted"], method)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    args = parser.parse_args()
    failures = []
    for transport in ("http", "sse"):
        with tempfile.TemporaryDirectory(prefix=f"dimcode-{transport}-") as directory:
            try:
                verify(str(args.executable.resolve()), transport, Path(directory))
            except AssertionError as error:
                failures.append(f"{transport}: {error}")
    if failures:
        raise SystemExit("\n".join(failures))


if __name__ == "__main__":
    main()
