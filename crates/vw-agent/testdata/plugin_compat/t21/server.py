# VibeWindow-authored fixture, project MIT license. No external service or private data.
import json
import pathlib
import sys
import time

payload = json.loads(pathlib.Path(__file__).with_name("result.json").read_text())
for line in sys.stdin:
    message = json.loads(line)
    if "id" not in message:
        continue
    method = message["method"]
    if method == "initialize":
        result = {"protocolVersion":"2024-11-05", "capabilities":{"tools":{}}, "serverInfo":{"name":"content-fixture","version":"1"}}
    elif method == "tools/list":
        result = {"tools":[{"name":"report","inputSchema":{"type":"object"}}]}
    elif method == "tools/call":
        arguments = message.get("params", {}).get("arguments", {})
        time.sleep(float(arguments.get("delay_ms", 0)) / 1000)
        result = json.loads(json.dumps(payload))
        result["_meta"]["source"] = sys.argv[1]
        result["structuredContent"]["source"] = sys.argv[1]
        result["isError"] = bool(arguments.get("fail", False))
        mode = arguments.get("mode")
        if mode == "empty":
            result = {"content":[], "_meta":{"source":sys.argv[1]}}
        elif mode == "structured":
            result = {"structuredContent":{"answer":42}, "_meta":{"source":sys.argv[1]}}
        elif mode == "structured-null":
            result = {"content":[], "structuredContent":None, "_meta":{"marker":"host-only-null"}}
        elif mode == "large":
            result["content"][0]["text"] = "L" * 600000
            result["content"][1]["data"] = "AAAA" * 150000
        elif mode == "link-size":
            result["content"][5]["size"] = 2**40
        elif mode == "collision":
            result["tools"] = []
            result["messages"] = []
            result["_meta"]["vibewindow/internal-wire-result"] = {"content":[], "isError":True}
        elif mode == "invalid-block":
            result["content"][1]["data"] = 10
        elif mode == "invalid-meta":
            result["content"][2]["_meta"] = "not an object"
        elif mode == "invalid-error":
            result["isError"] = "false"
        elif mode == "invalid-resource":
            result["content"][3]["resource"]["blob"] = "AAEC"
        elif mode == "unknown-content":
            result["content"][0]["type"] = "not-supported"
        elif mode == "notification":
            print(json.dumps({"jsonrpc":"2.0", "method":"notifications/fixture", "params":{"opaque":"not protocol content"}}), flush=True)
        elif mode == "invalid-notification":
            print(json.dumps({"jsonrpc":"2.0", "method":"notifications/progress", "params":{"progress":"invalid"}}), flush=True)
    else:
        result = {}
    print(json.dumps({"jsonrpc":"2.0","id":message["id"],"result":result}), flush=True)
