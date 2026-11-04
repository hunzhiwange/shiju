import json
import sys

TOOL = {
    "name": "Echo Tool",
    "description": "Echo a message from the MCP fixture.",
    "inputSchema": {
        "type": "object",
        "additionalProperties": False,
        "properties": {
            "message": {"type": "string"}
        },
        "required": ["message"]
    },
    "annotations": {
        "readOnlyHint": True,
        "destructiveHint": False,
        "idempotentHint": True
    }
}

def send(payload):
    sys.stdout.write(json.dumps(payload, separators=(",", ":")) + "\n")
    sys.stdout.flush()

for line in sys.stdin:
    if not line.strip():
        continue
    message = json.loads(line)
    method = message.get("method")
    request_id = message.get("id")
    if method == "initialize":
        params = message.get("params") or {}
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "protocolVersion": params.get("protocolVersion", "2025-06-18"),
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "fixture-mcp", "version": "1.0.0"},
                "instructions": "Fixture instructions for dynamic MCP tool tests."
            }
        })
    elif method == "tools/list":
        send({"jsonrpc": "2.0", "id": request_id, "result": {"tools": [TOOL]}})
    elif method == "tools/call":
        params = message.get("params") or {}
        arguments = params.get("arguments") or {}
        text = "echo:" + str(arguments.get("message", ""))
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "content": [{"type": "text", "text": text}],
                "structuredContent": {"echoed": arguments.get("message", "")},
                "isError": False
            }
        })
    elif request_id is not None:
        send({"jsonrpc": "2.0", "id": request_id, "error": {"code": -32601, "message": "Method not found"}})
