"""Exercise Snow ACP against a loopback model and an indexed, disposable RueJS copy."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("snow", type=Path)
parser.add_argument("rustcodegraph", type=Path)
parser.add_argument("ruejs_copy", type=Path)
args = parser.parse_args()
cwd = args.ruejs_copy.resolve()
if not (cwd / ".rustcodegraph").is_dir():
    parser.error("ruejs_copy must be a disposable indexed workspace")
tool_args = {
    "query": {"search": "createRueSiteApp"},
    "callers": {"symbol": "createRueSiteApp"},
    "node": {"symbol": "mountRueSiteApp"},
}
seen = []
name_prefix = "ruejs_graph_probe-rustcodegraph_"

class Model(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_POST(self):
        request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        names = {t["function"]["name"] for t in request.get("tools", [])}
        results = [m for m in request["messages"] if m["role"] == "tool"]
        seen.append((names, results))
        available = all(name_prefix + key in names for key in tool_args)
        keys = list(tool_args)
        if available and len(results) < len(keys):
            key = keys[len(results)]
            delta = {"tool_calls": [{"index": 0, "id": f"call_{key}", "type": "function", "function": {
                "name": name_prefix + key, "arguments": json.dumps(tool_args[key])
            }}]}
            reason = "tool_calls"
        else:
            delta = {"content": "RESULTS_OBSERVED" if available else "MCP_NOT_AVAILABLE"}
            reason = "stop"
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        for part, finish in [(delta, None), ({}, reason)]:
            chunk = dict(id="fixture", object="chat.completion.chunk", created=0, model="probe",
                         choices=[dict(index=0, delta=part, finish_reason=finish)])
            self.wfile.write(f"data: {json.dumps(chunk)}\n\n".encode())
        self.wfile.write(b"data: [DONE]\n\n")

with tempfile.TemporaryDirectory(prefix="snow-acp-") as private_home, \
        ThreadingHTTPServer(("127.0.0.1", 0), Model) as model:
    config = Path(private_home) / "snow"
    config.mkdir()
    threading.Thread(target=model.serve_forever, daemon=True).start()
    (config / "config.json").write_text(json.dumps({"snowcfg":{
        "baseUrl":f"http://127.0.0.1:{model.server_port}/v1", "baseUrlMode":"base", "apiKey":"fixture",
        "requestMethod":"chat", "advancedModel":"probe", "enablePromptOptimization":False, "maxRetries":0
    }}))
    env = dict(PATH=os.environ["PATH"], HOME=private_home, SNOW_CONFIG_DIR=str(config), SHIJU_SNOW_SKILL_ROOT="")
    with subprocess.Popen([args.snow.absolute(), "--acp"], cwd=cwd, env=env, text=True,
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
            for label, mount in [("before", []), ("mounted", servers), ("removed", []),
                                 ("load_mounted", servers), ("load_removed", []), ("denied", servers)]:
                start = len(seen)
                updates.clear()
                permissions.clear()
                if label.startswith("load_"):
                    rpc("session/load", dict(sessionId=session, cwd=str(cwd), mcpServers=mount))
                else:
                    session = rpc("session/new", dict(cwd=str(cwd), mcpServers=mount))["sessionId"]
                end = rpc("session/prompt", dict(sessionId=session, prompt=[dict(type="text",text="Run the fixture")]))
                names, results = seen[-1]
                completed = [u for u in updates if u.get("status") == "completed"]
                expected_tools = [name_prefix + key for key in tool_args] if mount else []
                assert permissions == expected_tools, (label, permissions)
                assert end["stopReason"] == "end_turn", (label, end)
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
                    started = [u.get("title") for u in updates if u["sessionUpdate"] == "tool_call"]
                    assert started == expected_tools
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
