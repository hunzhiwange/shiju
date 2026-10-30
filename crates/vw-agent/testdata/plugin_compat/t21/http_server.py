# Real loopback HTTP/Streamable HTTP/SSE protocol fixture. Project MIT license.
import http.server
import json
import pathlib
import queue
import sys

mode = sys.argv[1]
events = queue.Queue()
payload = json.loads(pathlib.Path(__file__).with_name("result.json").read_text())


def response(message):
    if "id" not in message:
        return None
    method = message["method"]
    if method == "initialize":
        result = {"protocolVersion":"2025-03-26", "capabilities":{"tools":{}}, "serverInfo":{"name":"network-fixture","version":"1"}}
    elif method == "tools/list":
        result = {"tools":[{"name":"report","inputSchema":{"type":"object"}}]}
    elif method == "tools/call":
        result = json.loads(json.dumps(payload))
        result["_meta"]["source"] = mode
        result["structuredContent"]["source"] = mode
        result["isError"] = bool(message.get("params",{}).get("arguments",{}).get("fail", False))
    else:
        result = {}
    return {"jsonrpc":"2.0","id":message["id"],"result":result}


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *_):
        pass

    def empty(self, code):
        self.send_response(code)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_POST(self):
        message = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        result = response(message)
        if self.path == "/messages":
            if result is not None:
                events.put(result)
            self.empty(202)
        elif result is None:
            self.empty(202)
        else:
            body = json.dumps(result)
            if mode == "http-sse":
                body = "event: message\ndata: " + body + "\n\n"
                kind = "text/event-stream"
            else:
                kind = "application/json"
            body = body.encode()
            self.send_response(200)
            self.send_header("Content-Type", kind)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            self.wfile.flush()

    def do_GET(self):
        if self.path != "/sse":
            return self.empty(405)
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()
        self.wfile.write(b"event: endpoint\ndata: /messages\n\n")
        self.wfile.flush()
        try:
            while True:
                message = events.get()
                self.wfile.write(("event: message\ndata: " + json.dumps(message) + "\n\n").encode())
                self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            pass

    def do_DELETE(self):
        self.empty(204)


server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
print(server.server_address[1], flush=True)
server.serve_forever()
