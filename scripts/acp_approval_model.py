"""Deterministic tool inputs for the real ACP approval matrix."""
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import secrets
import shlex
import threading
from urllib.parse import urlsplit

from acp_skill_mount_claude import messages_wire
from acp_skill_mount_codex import responses_wire

CONTENTS = ("first edit", "second edit")
DONE = "ACP_APPROVAL_FIXTURE_DONE"
WRITE_TOOLS = ("write", "write_file", "write_to_file", "write_text_file", "create_file", "create")
SHELL_TOOLS = ("bash", "shell", "run_shell_command", "exec_command", "shell_command", "execute_command")


def shell_command(index, workspace, outside):
    target = workspace / "note.txt" if index < 2 else outside
    if index < 3:
        content = CONTENTS[index] if index < 2 else "outside edit"
        return "printf '%s' " + shlex.quote(content) + " > " + shlex.quote(str(target))
    if index == 3:
        return "touch " + shlex.quote(str(workspace / "terminal.txt"))
    return "rm " + shlex.quote(str(workspace / "keep.txt"))


def tool_inputs(tools, index, workspace, outside):
    """Issue calls only against the adapter's actually advertised tools."""
    definitions = {tool["name"].lower(): tool for item in tools
                   if (tool := item.get("function", item)).get("name")}
    if index < 3:
        target = workspace / "note.txt" if index < 2 else outside
        content = CONTENTS[index] if index < 2 else "outside edit"
        for name in WRITE_TOOLS:
            if name not in definitions:
                continue
            tool = definitions[name]
            props = properties(tool)
            path_key = next((key for key in ("file_path", "filePath", "path", "absolute_path")
                             if key in props), None)
            if path_key and "content" in props:
                return tool["name"], {path_key: str(target), "content": content}
    for name in SHELL_TOOLS:
        if name not in definitions:
            continue
        tool = definitions[name]
        props = properties(tool)
        command_key = next((key for key in ("command", "cmd") if key in props), None)
        if not command_key:
            continue
        arguments = {command_key: shell_command(index, workspace, outside)}
        if "description" in props:
            arguments["description"] = "Run the isolated approval fixture operation"
        if "workdir" in props:
            arguments["workdir"] = str(workspace)
        if "timeout" in props:
            arguments["timeout"] = 5000
        return tool["name"], arguments
    return None


def properties(tool):
    schema = next((tool[key] for key in ("parameters", "input_schema", "parametersJsonSchema")
                   if key in tool), {})
    return schema.get("properties", {})


def matches_workspace_edit(arguments, workspace):
    if not isinstance(arguments, dict):
        return False
    path = next((arguments[key] for key in ("file_path", "filePath", "path", "absolute_path")
                 if key in arguments), None)
    if isinstance(path, str) and (workspace / Path(path)) == workspace / "note.txt":
        return arguments.get("content") in CONTENTS
    return (arguments.get("command") or arguments.get("cmd")) in [
        shell_command(index, workspace, workspace / "unused") for index in range(2)
    ]


def chat_wire(message, model, response_id):
    finish = "tool_calls" if message.get("tool_calls") else "stop"
    usage = {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    result = {"id": response_id, "object": "chat.completion", "model": model, "usage": usage,
              "choices": [{"index": 0, "message": message, "finish_reason": finish}]}
    delta = dict(message)
    if delta.get("tool_calls"):
        delta["tool_calls"] = [dict(delta["tool_calls"][0], index=0)]
    frame = {"id": response_id, "object": "chat.completion.chunk", "model": model}
    events = [
        dict(frame, choices=[{"index": 0, "delta": delta, "finish_reason": None}]),
        dict(frame, choices=[{"index": 0, "delta": {}, "finish_reason": finish}], usage=usage),
    ]
    return result, ("".join("data: " + json.dumps(event) + "\n\n" for event in events)
                    + "data: [DONE]\n\n").encode()


@contextmanager
def model_fixture(workspace, outside):
    """Reuse existing wire envelopes while driving native tools with fixed inputs."""
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def reply(self, data, content_type="application/json"):
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def authenticated(self):
            return fixture.key in (self.headers.get("x-api-key"), self.headers.get("x-goog-api-key"),
                                   self.headers.get("Authorization", "").removeprefix("Bearer "))

        def do_GET(self):
            if urlsplit(self.path).path != "/v1/models" or not self.authenticated():
                return self.send_error(404)
            self.reply(json.dumps({"object": "list", "data": [
                {"id": model, "object": "model"} for model in fixture.models
            ]}).encode())

        def do_POST(self):
            length = int(self.headers.get("Content-Length", "0"))
            if not 0 < length < 2_000_000 or not self.authenticated():
                return self.send_error(400)
            body = json.loads(self.rfile.read(length))
            route = urlsplit(self.path).path
            model = route.rsplit("/", 1)[-1].split(":", 1)[0] if route.startswith("/v1beta/") else body.get("model")
            if model not in fixture.models:
                return self.send_error(400)
            if route in ("/v1/messages/count_tokens", "/v1/chat/completions/count_tokens"):
                return self.reply(b'{"input_tokens":1}')
            if route.endswith(":countTokens"):
                return self.reply(b'{"totalTokens":1}')
            if route not in ("/v1/messages", "/v1/responses", "/v1/chat/completions") and not (
                route.startswith("/v1beta/models/") and
                route.endswith((":generateContent", ":streamGenerateContent"))
            ):
                return self.send_error(404)
            tools = body.get("tools", [])
            gemini = route.startswith("/v1beta/")
            if gemini:
                tools = [tool for item in tools for tool in item.get("functionDeclarations", [])]
            message = {"role": "assistant", "content": DONE}
            if route == "/v1/responses" and body.get("text", {}).get("format", {}).get("schema"):
                message["content"] = '{"title":"ACP approval fixture"}'
            elif tools:
                call = fixture.next_tool(tools, body)
                if call:
                    message.update(content=None, tool_calls=[call])
            if gemini:
                part = {"text": message["content"]}
                if message.get("tool_calls"):
                    call = message["tool_calls"][0]
                    function = call["function"]
                    part = {"functionCall": {"id": call["id"], "name": function["name"],
                                             "args": json.loads(function["arguments"])}}
                result = {"candidates": [{"content": {"role": "model", "parts": [part]},
                                          "index": 0, "finishReason": "STOP"}],
                          "usageMetadata": {"promptTokenCount": 1, "candidatesTokenCount": 1,
                                            "totalTokenCount": 2}}
                wire = ("data: " + json.dumps(result) + "\n\n").encode()
            elif route == "/v1/messages":
                result, wire = messages_wire(message, body["model"])
            elif route == "/v1/responses":
                result, wire = responses_wire(message, body["model"])
            else:
                result, wire = chat_wire(message, body["model"], "fixture-" + secrets.token_hex(8))
            if body.get("stream") or route.endswith(":streamGenerateContent"):
                return self.reply(wire, "text/event-stream")
            self.reply(json.dumps(result).encode())

    class Fixture(ThreadingHTTPServer):
        daemon_threads = True
        key = "offline-acp-approval-key"
        models = ("acp-fixture-primary",)

        def reset(self, phase, operation=None):
            with self.lock:
                self.phase = phase
                self.operation = operation
                self.actions = []
                self.issued = set()
                self.results = {}
                self.tool_names = set()
                self.workspace_edit_issued = False

        def next_tool(self, tools, request):
            with self.lock:
                for turn in request.get("messages", []):
                    if turn.get("role") == "tool" and turn.get("tool_call_id") in self.issued:
                        self.results[turn["tool_call_id"]] = None
                    content = turn.get("content")
                    if not isinstance(content, list):
                        continue
                    for block in content:
                        if block.get("type") == "tool_result" and block.get("tool_use_id") in self.issued:
                            self.results[block["tool_use_id"]] = block.get("is_error", False)
                for item in request.get("input", []):
                    if isinstance(item, dict) and item.get("type") == "function_call_output":
                        if item.get("call_id") in self.issued:
                            self.results[item["call_id"]] = None
                for turn in request.get("contents", []):
                    for part in turn.get("parts", []):
                        response = part.get("functionResponse", {})
                        if response.get("id") in self.issued:
                            self.results[response["id"]] = None
                index = len(self.actions)
                if index >= (5 if self.operation is None else 1):
                    return None
                self.tool_names.update(tool.get("function", tool).get("name", "") for tool in tools)
                selected = tool_inputs(tools, index if self.operation is None else self.operation, workspace, outside)
                if selected is None:
                    return None
                name, arguments = selected
                self.workspace_edit_issued = matches_workspace_edit(arguments, workspace)
                tool_id = f"toolu_approval_{self.phase}_{index}"
                self.issued.add(tool_id)
                self.actions.append(name)
                return {"id": tool_id, "type": "function",
                        "function": {"name": name, "arguments": json.dumps(arguments)}}

    with Fixture(("127.0.0.1", 0), Handler) as fixture:
        fixture.url = f"http://127.0.0.1:{fixture.server_port}"
        fixture.lock = threading.Lock()
        fixture.reset("startup")
        thread = threading.Thread(target=fixture.serve_forever)
        thread.start()
        try:
            yield fixture
        finally:
            fixture.shutdown()
            thread.join()
