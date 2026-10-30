# Authored for VibeWindow; project license applies. All output/state stays in test temp roots.
import json
import os
import sys
import time

marker = sys.argv[1]
mode = sys.argv[2] if len(sys.argv) > 2 else "normal"
with open(marker, "a") as output:
    output.write(str(os.getpid()) + "\n")
for line in sys.stdin:
    message = json.loads(line)
    if "id" not in message:
        continue
    method = message.get("method")
    if method == "initialize":
        if mode == "startup-hang":
            time.sleep(30)
        result = {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "serverInfo": {"name": "session-fixture", "version": "1"}}
    elif method == "tools/list":
        if mode == "catalog-hang":
            time.sleep(30)
        result = {"tools": [{"name": "pid", "description": str(os.getpid()), "inputSchema": {"type": "object"}}]}
    elif method == "tools/call":
        if message.get("params", {}).get("name") == "hang":
            with open(marker + ".call", "w") as output:
                output.write("started")
            time.sleep(30)
        result = {"content": [{"type": "text", "text": str(os.getpid())}]}
    else:
        result = {}
    print(json.dumps({"jsonrpc": "2.0", "id": message["id"], "result": result}), flush=True)
