//! Protocol fixtures, shared by native MCP acceptance scenarios.
pub const RUNTIME_SERVER: &str = r#"
import json
import os
import sys

for line in sys.stdin:
    request = json.loads(line)
    if "id" not in request:
        continue
    method = request["method"]
    if method == "initialize":
        result = {
            "protocolVersion": request["params"]["protocolVersion"],
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "fixture", "version": "1"},
        }
    elif method == "tools/list":
        result = {"tools": [{"name": "echo", "inputSchema": {"type": "object"}, "annotations": {"readOnlyHint": True}}]}
    elif method == "tools/call":
        text = request["params"]["arguments"]["message"] + "|" + os.path.basename(os.getcwd())
        result = {"content": [{"type": "text", "text": text}]}
    else:
        result = {}
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
"#;
pub const REVOCATION_SERVER: &str = r#"
import json
import os
import sys
import time

for line in sys.stdin:
    request = json.loads(line)
    if "id" not in request:
        continue
    method = request["method"]
    if method == "initialize":
        result = {
            "protocolVersion": request["params"]["protocolVersion"],
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "fixture", "version": "1"},
        }
    elif method == "tools/list":
        result = {"tools": [{"name": "read", "inputSchema": {"type": "object"}, "annotations": {"readOnlyHint": True}}]}
    elif method == "tools/call":
        with open(sys.argv[1], "a") as calls:
            calls.write(sys.argv[2] + "\n")
        if request["params"]["arguments"].get("wait"):
            while not os.path.exists(sys.argv[1] + ".release"):
                time.sleep(0.01)
        result = {"content": [{"type": "text", "text": sys.argv[2]}]}
    else:
        result = {}
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
"#;
