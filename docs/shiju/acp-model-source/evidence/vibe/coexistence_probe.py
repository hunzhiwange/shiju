#!/usr/bin/env python3
"""Drive one mounted Vibe ACP session with an isolated model and RustCodeGraph."""
import argparse
from hashlib import sha256
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import queue
import shutil
import subprocess
import tempfile
import threading
from urllib.parse import urlsplit

import tomli_w

SEARCH = "search_tool_functions"
PROGRAM = "run_typescript"
CONNECTOR = "ruejs_graph_probe"
GRAPH_TOOL = "rustcodegraph_query"
FUNCTION = f"mcp_{CONNECTOR}.{GRAPH_TOOL}"
GRAPH_RESULT = "createRueSiteApp  app/app.tsx:32"
NATIVE_RESULT = "VIBE_NATIVE_READ_CHALLENGE"


def sandbox(port, root, runtime, python, graph, project):
    def literal(path):
        return json.dumps(str(path))

    # Block system credential IPC and child execution independently of the
    # fixture's keyring settings. The exceptions are the venv and stdio MCP.
    return (
        "(version 1)(allow default)"
        "(deny mach-lookup)(deny mach-register)(deny appleevent-send)"
        "(deny ipc-posix-shm*)(deny ipc-posix-sem*)"
        f"(deny network*)(allow network-outbound (remote tcp \"localhost:{port}\"))"
        "(deny process-exec)"
        f"(allow process-exec (literal {literal(python)}) (literal {literal(graph)}))"
        '(deny file-read-data (subpath "/Users"))'
        f'(allow file-read-data (subpath {literal(python.parent.parent)})'
        f' (subpath {literal(runtime)}) (subpath {literal(project)}) (literal {literal(graph)}))'
        f"(deny file-write*)(allow file-write* (subpath {literal(root)}) (literal \"/dev/null\"))"
        '(deny file-read* (subpath "/private/etc/ssh") (subpath "/System/Library/Keychains")'
        ' (regex #"/(Keychains|keychains)(/|$)"))'
    )


def serve(state):
    class Model(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            tools = [tool["function"]["name"] for tool in body.get("tools", [])]
            results = {msg["tool_call_id"]: msg["content"] for msg in body["messages"]
                       if msg["role"] == "tool"}
            state["model_rounds"].append({"tools": tools, "tool_results": results})
            calls = []
            if not results and SEARCH in tools and PROGRAM in tools:
                calls = [("probe-discovery", SEARCH, {
                    "mode": "details", "functions": ["file_system.read_file", FUNCTION],
                })]
            elif "probe-discovery" in results and not {"probe-native", "probe-mcp"} & results.keys():
                calls = [
                    ("probe-native", PROGRAM, {
                        "code": "async function main() { return await tools.file_system.read_file({path: 'challenge.txt'}); }",
                    }),
                    ("probe-mcp", PROGRAM, {
                        "code": f"async function main() {{ return await tools.{FUNCTION}({{search: 'createRueSiteApp'}}); }}",
                    }),
                ]
            delta = {"role": "assistant", "content": "PROBE_REPLY"}
            if calls:
                delta = {"role": "assistant", "tool_calls": [
                    {"index": index, "id": ident, "type": "function",
                     "function": {"name": name, "arguments": json.dumps(arguments)}}
                    for index, (ident, name, arguments) in enumerate(calls)
                ]}
            frame = {"id": "chatcmpl-probe", "object": "chat.completion.chunk",
                     "created": 0, "model": "probe-model"}
            chunks = [
                {**frame, "choices": [{"index": 0, "delta": delta, "finish_reason": None}]},
                {**frame, "choices": [{"index": 0, "delta": {},
                                      "finish_reason": "tool_calls" if calls else "stop"}]},
                {**frame, "choices": [], "usage": {"prompt_tokens": 30, "completion_tokens": 3,
                                                  "total_tokens": 33}},
            ]
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            for chunk in chunks:
                self.wfile.write(f"data: {json.dumps(chunk)}\n\n".encode())
            self.wfile.write(b"data: [DONE]\n\n")

    return ThreadingHTTPServer(("127.0.0.1", 0), Model)


def run(args, root, model_url, state):
    entry = args.entry.absolute()
    runtime = entry.parent.parent.resolve()
    python = runtime / "bin/python"
    graph = args.graph.resolve(strict=True)
    # SQLite may need writable sidecars even when the MCP query is read-only.
    project = root / "graph-project"
    shutil.copytree(args.project, project, symlinks=True)
    port = urlsplit(model_url).port

    workspace = root / "workspace"
    workspace.mkdir()
    (workspace / "challenge.txt").write_text(NATIVE_RESULT)
    route = {"name": "probe-model", "alias": "session-selected", "provider": "session-provider",
             "max_context_length": 262144, "max_output_tokens": 1024}
    binding = root / "session-model.toml"
    binding.touch(mode=0o600)
    binding.write_text(tomli_w.dumps({
        "active_model": "session-selected", "allowed_models": [route["name"]],
        "models": [route], "compaction_model": route,
        "providers": [{"name": "session-provider", "api_base": model_url + "/v1",
                       "api_key_env_var": "VW_VIBE_RELAY_TOKEN", "api_style": "openai",
                       "backend": "generic"}],
        "utility_models": {"title": "session-selected", "smart_approve": "session-selected"},
    }))
    (root / "config.toml").write_text("enable_telemetry = false\n")
    env = {"PATH": "/usr/bin:/bin", "HOME": str(root), "TMPDIR": str(root), "VIBE_HOME": str(root),
           "SHIJU_VIBE_SKILL_ROOT": "", "VW_VIBE_RELAY_TOKEN": "local-fixture-only",
           "VW_ACP_LAUNCH_REFERENCE": "vw-session-model:" + sha256(str(root).encode()).hexdigest(),
           "VIBE_TEST_DISABLE_KEYRING": "1", "PYTHON_KEYRING_BACKEND": "keyring.backends.null.Keyring"}
    command = ["/usr/bin/sandbox-exec", "-p", sandbox(port, root, runtime, python.resolve(), graph, project),
               str(python), str(entry), "--session-model-config", str(binding)]
    state["entry_sha256"] = sha256(entry.read_bytes()).hexdigest()
    with args.output.with_suffix(".stderr").open("w") as stderr, subprocess.Popen(
        command, cwd=workspace, env=env, stdin=subprocess.PIPE,
        stdout=subprocess.PIPE, stderr=stderr, text=True,
    ) as proc:
        replies = queue.Queue()

        def receive():
            for line in proc.stdout:
                replies.put(json.loads(line))
            replies.put(None)

        reader = threading.Thread(target=receive, daemon=True)
        reader.start()

        def send(message):
            proc.stdin.write(json.dumps({"jsonrpc": "2.0", **message}) + "\n")
            proc.stdin.flush()

        def rpc(ident, method, params):
            send({"id": ident, "method": method, "params": params})
            while True:
                msg = replies.get(timeout=40)
                if msg is None:
                    raise RuntimeError("managed ACP process exited")
                if msg.get("method") == "session/update":
                    update = msg["params"]["update"]
                    if update["sessionUpdate"].startswith("tool_call"):
                        state["acp_tools"].append(update)
                    if update["sessionUpdate"] == "agent_message_chunk":
                        state["agent_messages"].append(update["content"].get("text", ""))
                if msg.get("method") == "session/request_permission":
                    call = msg["params"]["toolCall"]
                    if msg["params"]["sessionId"] != state["session_id"] or not any(
                        event["toolCallId"] == call["toolCallId"]
                        and event.get("_meta", {}).get("tool_name") == FUNCTION
                        for event in state["acp_tools"]
                    ):
                        raise RuntimeError("unexpected permission request")
                    state["permissions"].append(call)
                    option = next(option for option in msg["params"]["options"]
                                  if option["kind"] == "allow_once")
                    send({"id": msg["id"], "result": {
                        "outcome": {"outcome": "selected", "optionId": option["optionId"]},
                    }})
                if msg.get("id") == ident and "method" not in msg:
                    if "error" in msg:
                        raise RuntimeError(str(msg["error"]))
                    return msg["result"]

        try:
            info = rpc(1, "initialize", {"protocolVersion": 1, "clientCapabilities": {}})
            assert info["agentInfo"]["version"] == "3.0.4", info
            session = rpc(2, "session/new", {"cwd": str(workspace), "mcpServers": [{
                "name": CONNECTOR, "command": str(graph), "env": [],
                "args": ["serve", "--mcp", "--path", str(project), "--no-watch"],
            }]})
            state["session_id"] = session["sessionId"]
            result = rpc(3, "session/prompt", {"sessionId": session["sessionId"], "prompt": [{
                "type": "text", "text": "Discover and execute the native read and mounted RustCodeGraph query.",
            }]})
            state["stop_reason"] = result["stopReason"]
        finally:
            proc.stdin.close()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
            state["exit_status"] = proc.returncode
            reader.join(timeout=5)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--entry", type=Path, required=True)
    parser.add_argument("--graph", type=Path, required=True)
    parser.add_argument("--project", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.parent.stat().st_mode & 0o077:
        parser.error("--output must be inside a private directory")
    state = {"model_rounds": [], "acp_tools": [], "permissions": [], "agent_messages": []}
    with tempfile.TemporaryDirectory(prefix="vibe-coexistence-") as directory, serve(state) as server:
        root = Path(directory).resolve()
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            run(args, root, f"http://127.0.0.1:{server.server_port}", state)
            assert state["stop_reason"] == "end_turn" and state["exit_status"] == 0
            tool_rounds = [round_ for round_ in state["model_rounds"] if round_["tools"]]
            tools = tool_rounds[0]["tools"]
            assert SEARCH in tools and PROGRAM in tools, tools
            results = tool_rounds[-1]["tool_results"]
            assert f"function: {FUNCTION}" in results["probe-discovery"]
            assert NATIVE_RESULT in results["probe-native"]
            assert GRAPH_RESULT in results["probe-mcp"]
            completed = {event.get("_meta", {}).get("tool_name"): event
                         for event in state["acp_tools"] if event.get("status") == "completed"}
            assert {SEARCH, "read_file", FUNCTION} <= completed.keys()
            assert [call["toolCallId"] for call in state["permissions"]] == [
                completed[FUNCTION]["toolCallId"],
            ]
            assert "PROBE_REPLY" in "".join(state["agent_messages"])
            state["passed"] = True
        finally:
            server.shutdown()
            thread.join()
            for directory in (root, *root.rglob("*")):
                if directory.is_dir() and not directory.is_symlink():
                    directory.chmod(0o700)
            args.output.write_text(json.dumps(state, indent=2) + "\n")


if __name__ == "__main__":
    main()
