"""MIT; repository-authored MCP 2025-06-18 form fixture, no private services."""
import json
import os
import sys
import threading

initialize = None
outer_id = None
early = False
redact_echo = False
output_lock = threading.Lock()

def send(value):
    with output_lock:
        print(json.dumps(value), flush=True)

def later(callback):
    timer = threading.Timer(0.05, callback)
    timer.daemon = True
    timer.start()

for line in sys.stdin:
    msg = json.loads(line)
    method = msg.get("method")
    if method == "initialize":
        initialize = msg["params"]
        send({"jsonrpc": "2.0", "id": msg["id"], "result": {
            "protocolVersion": "2025-06-18", "capabilities": {"tools": {}},
            "serverInfo": {"name": "form-fixture", "version": "1"}}})
    elif method == "tools/list":
        send({"jsonrpc": "2.0", "id": msg["id"], "result": {"tools": [
            {"name": "form", "inputSchema": {"type": "object"}}]}})
    elif method == "tools/call":
        outer_id = msg["id"]
        args = msg["params"].get("arguments", {})
        schema = args.get("schema", {
            "type": "object", "properties": {
                "name": {"type": "string", "minLength": 2, "maxLength": 20},
                "age": {"type": "integer", "minimum": 18, "maximum": 120},
                "approved": {"type": "boolean"}}, "required": ["name", "age", "approved"]})
        params = args.get("params", {"message": "请确认公开服务表单", "requestedSchema": schema})
        early = args.get("complete_early", False)
        redact_echo = args.get("suppress_reply_content", False)
        send({"jsonrpc": "2.0", "id": "form-1", "method": "elicitation/create", "params": params})
        if args.get("cancel"):
            later(lambda: send({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": "form-1"}}))
        if args.get("disconnect"):
            later(lambda: os._exit(0))
        if early:
            current = outer_id
            later(lambda: send({"jsonrpc":"2.0", "id":current, "result":{"content":[], "structuredContent":{"early":True}}}))
    elif msg.get("id") == "form-1":
        if not early:
            if redact_echo:
                msg = {"result": {"action":msg.get("result", {}).get("action"),
                    "received_private_value":msg.get("result", {}).get("content", {}).get("name") == "user-private-value"}}
            send({"jsonrpc": "2.0", "id": outer_id, "result": {"content": [],
                "structuredContent": {"form_reply": msg, "initialize": initialize, "pid":os.getpid()}}})
