"""MIT-licensed local fixture for capability negotiation and reverse requests.

Independent JSON-RPC server, no network or host data access. Writes only to stdout.
"""
import json
import os
import sys

version = sys.argv[1]
initialize = None
pending = {}
call_id = None


def send(value):
    print(json.dumps({"jsonrpc": "2.0", **value}), flush=True)


for line in sys.stdin:
    message = json.loads(line)
    method = message.get("method")
    if method == "initialize":
        initialize = message["params"]
        send({"id": message["id"], "result": {
            "protocolVersion": version,
            "capabilities": {"tools": {}, "experimental": {
                "openai/elicitation": {"userVerification": True},
                "openai/network-policy": {}}},
            "serverInfo": {"name": "codex_apps", "version": "1"}}})
    elif method == "tools/list":
        send({"id": message["id"], "result": {"tools": [{
            "name": "probe", "inputSchema": {"type": "object"}}]}})
    elif method == "tools/call":
        call_id = message["id"]
        for key, request, params in [
            ("roots", "roots/list", {}),
            ("sampling", "sampling/createMessage", {"messages": [], "maxTokens": 1}),
            ("elicitation", "elicitation/create", {
                "message": "fixture", "requestedSchema": {"type": "object", "properties": {}}}),
        ]:
            send({"id": key, "method": request, "params": params})
    elif method == "ping":
        send({"id": message["id"], "result": {}})
    elif method is None and message.get("id") in ("roots", "sampling", "elicitation"):
        pending[message["id"]] = message
        if len(pending) == 3:
            send({"id": call_id, "result": {"content": [{"type": "text", "text": "probe"}],
                "structuredContent": {"initialize": initialize, "replies": pending, "pid": os.getpid()}}})
            pending = {}
