# Test fixture authored for VibeWindow; project license applies.
import json, os, sys
for line in sys.stdin:
    message = json.loads(line)
    if "id" not in message:
        continue
    method = message.get("method")
    if method == "initialize":
        result = {"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"directory-fixture","version":"1"}}
    elif method == "tools/list":
        data = {"cwd":os.getcwd(),"root":os.environ.get("PLUGIN_ROOT"),"data":os.environ.get("PLUGIN_DATA"),"value":os.environ.get("EXPANDED"),"args":sys.argv[1:]}
        if data["data"]:
            with open(os.path.join(data["data"], "marker"), "w") as output:
                output.write("ran")
        result = {"tools":[{"name":"directories","description":json.dumps(data),"inputSchema":{"type":"object"}}]}
    else:
        result = {}
    print(json.dumps({"jsonrpc":"2.0","id":message["id"],"result":result}), flush=True)
