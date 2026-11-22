"""Exercise patched Dirac ACP against a loopback model and an indexed, disposable RueJS copy."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("dirac", type=Path)
parser.add_argument("rustcodegraph", type=Path)
parser.add_argument("ruejs_copy", type=Path)
parser.add_argument("--plain-text", action="store_true")
args = parser.parse_args()
cwd = args.ruejs_copy.resolve()
if not (cwd / ".rustcodegraph").is_dir():
    parser.error("ruejs_copy must be a disposable indexed workspace")
tool_args = {
    "query": {"search": "createRueSiteApp"},
    "callers": {"symbol": "createRueSiteApp"},
    "node": {"name": "mountRueSiteApp"},
}
seen = []
truncated_once = False
name_prefix = "ruejs_graph_probe-rustcodegraph_"

class Model(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_POST(self):
        global truncated_once
        request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        names = {t["function"]["name"] for t in request.get("tools", [])}
        results = [m for m in request["messages"] if m["role"] == "tool"]
        seen.append((names, results, request["messages"]))
        available = all(name_prefix + key in names for key in tool_args)
        keys = list(tool_args)
        if available and len(results) < len(keys):
            key = keys[len(results)]
            delta = {"tool_calls": [{"index": 0, "id": f"call_{key}", "type": "function", "function": {
                "name": name_prefix + key, "arguments": json.dumps(tool_args[key])
            }}]}
            reason = "tool_calls"
        elif label == "truncated" and not truncated_once:
            truncated_once = True
            delta = {"content": "RESPONSE_TRUNCATED"}
            reason = "length"
        elif args.plain_text:
            answer = "RESULTS_OBSERVED" if available else "MCP_NOT_AVAILABLE"
            delta = {"content": answer[:5]}
            reason = "stop"
        else:
            delta = {"tool_calls": [{"index": 0, "id": "call_respond", "type": "function", "function": {
                "name": "respond", "arguments": json.dumps({"operation":"complete","text":"RESULTS_OBSERVED" if available else "MCP_NOT_AVAILABLE"})
            }}]}
            reason = "tool_calls"
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        parts = [(delta, None)]
        if reason == "stop":
            parts.append(({"content": answer[5:]}, None))
        for part, finish in [*parts, ({}, reason)]:
            chunk = dict(id="fixture", object="chat.completion.chunk", created=0, model="probe",
                         choices=[dict(index=0, delta=part, finish_reason=finish)])
            if finish:
                chunk["usage"] = {"prompt_tokens": 20, "completion_tokens": 8, "total_tokens": 28}
            self.wfile.write(f"data: {json.dumps(chunk)}\n\n".encode())
        self.wfile.write(b"data: [DONE]\n\n")

with tempfile.TemporaryDirectory(prefix="dirac-acp-") as private_home, \
        ThreadingHTTPServer(("127.0.0.1", 0), Model) as model:
    config = Path(private_home) / "dirac"
    config.mkdir()
    if args.plain_text:
        (config / "data").mkdir()
        (config / "data/globalState.json").write_text(json.dumps({"toolToggles": {"respond": False}}))
    threading.Thread(target=model.serve_forever, daemon=True).start()
    env = dict(PATH=os.environ["PATH"], HOME=private_home, DIRAC_DIR=str(config), DIRAC_DATA_DIR=str(config), DIRAC_PROVIDER="openai", DIRAC_MODEL="probe", DIRAC_API_KEY="fixture", DIRAC_BASE_URL=f"http://127.0.0.1:{model.server_port}/v1")
    with subprocess.Popen(["node", args.dirac.absolute(), "--acp", "--config", str(config)], cwd=cwd, env=env, text=True,
                          stdin=subprocess.PIPE, stdout=subprocess.PIPE) as proc:
        deadline = threading.Timer(45, proc.terminate)
        deadline.start()
        rpc_id = 0
        updates = []
        permissions = []
        def send(packet):
            print(json.dumps(dict(jsonrpc="2.0", **packet)), file=proc.stdin, flush=True)

        def rpc(method, params):
            global rpc_id
            rpc_id += 1
            send(dict(id=rpc_id, method=method, params=params))
            for line in proc.stdout:
                packet = json.loads(line)
                if packet.get("method") == "session/request_permission":
                    permissions.append(packet["params"]["toolCall"]["title"])
                    if label == "cancelled":
                        send(dict(method="session/cancel", params={"sessionId": packet["params"]["sessionId"]}))
                        continue
                    choice = "reject_once" if label == "denied" else "approve_once"
                    send(dict(id=packet["id"], result={"outcome":{"outcome":"selected","optionId":choice}}))
                elif packet.get("method") == "session/update":
                    updates.append(packet["params"]["update"])
                elif packet.get("id") == rpc_id:
                    if "error" in packet:
                        raise RuntimeError(packet["error"])
                    return packet["result"]
            raise RuntimeError("ACP disconnected")

        try:
            print(rpc("initialize", dict(protocolVersion=1,clientCapabilities={})), flush=True)
            graph = str(args.rustcodegraph.resolve())
            servers = [dict(name="ruejs_graph_probe", command=graph,
                            args=["serve","--mcp","--path",str(cwd),"--no-watch"],env=[])]
            scenarios = [("before", []), ("mounted", servers), ("removed", []),
                         ("denied", servers), ("cancelled", servers)]
            if args.plain_text:
                scenarios.append(("truncated", []))
            for label, mount in scenarios:
                start = len(seen)
                updates.clear()
                permissions.clear()
                session = rpc("session/new", dict(cwd=str(cwd), mcpServers=mount))["sessionId"]
                end = rpc("session/prompt", dict(sessionId=session, prompt=[dict(type="text",text="Run the fixture")]))
                names, results, _ = seen[-1]
                completed = [u for u in updates if u.get("status") == "completed" and u.get("title","").startswith(name_prefix)]
                assert len(seen) > start, (label, "no model requests")
                expected_tools = [name_prefix + key for key in tool_args] if mount else []
                if label == "cancelled":
                    assert end["stopReason"] == "cancelled", end
                    assert permissions == expected_tools[:1] and not completed and not results
                    print("cancelled: approval interrupted before execution", flush=True)
                    continue
                assert permissions == expected_tools, (label, permissions)
                assert end["stopReason"] == "end_turn", (label, end)
                if args.plain_text:
                    expected_text = "RESULTS_OBSERVED" if mount else "MCP_NOT_AVAILABLE"
                    answer = "".join(u["content"]["text"] for u in updates if u["sessionUpdate"] == "agent_message_chunk")
                    assert answer == expected_text, (label, answer)
                    assert all("respond" not in inventory for inventory, _, _ in seen[start:])
                    assert not any(u.get("title") in {"Task Completed", "Task Error"} for u in updates)
                    expected_requests = {"truncated": 2}.get(label, 4 if mount else 1)
                    assert len(seen) - start == expected_requests
                print(f"{label}: {len(seen)-start} model requests, {len(completed)} completed tools", flush=True)
                if label == "denied":
                    assert len(results) == len(permissions) == 3 and not completed
                    assert all(r["content"] == "Permission denied by user" for r in results)
                elif mount:
                    assert len(results) == len(completed) == len(permissions) == 3
                    for required, message in zip(("createRueSiteApp","mountRueSiteApp","mountRueSiteApp"), results):
                        assert required in message["content"], message["content"][:200]
                    assert [u.get("title") for u in completed] == expected_tools
                    assert all(u.get("rawOutput") for u in completed)
                    started = [u.get("title") for u in updates if u["sessionUpdate"] == "tool_call" and u.get("title","").startswith(name_prefix)]
                    assert started == expected_tools
                    reentries = [
                        ("session/load", {"sessionId": session, "cwd": str(cwd), "mcpServers": []}),
                        ("session/resume", {"sessionId": session, "cwd": str(cwd), "mcpServers": servers}),
                    ]
                    if sys.platform == "linux":
                        reentries.extend((method, {"sessionId": session, "cwd": str(cwd), "mcpServers": []})
                                         for method in ("session/resume", "session/fork"))
                    for method, params in reentries:
                        try:
                            rpc(method, params)
                        except RuntimeError as error:
                            assert "supports only new sessions" in str(error), error
                        else:
                            raise AssertionError(f"{method} accepted an unsupported MCP reattachment")
                else:
                    assert not any(n.startswith(name_prefix) for n in names)
                    assert not results and not permissions and not completed
        finally:
            deadline.cancel()
            proc.stdin.close()
            try:
                proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                proc.terminate()
                proc.wait(timeout=3)
    model.shutdown()
