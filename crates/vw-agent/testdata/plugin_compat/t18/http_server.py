# Authored for VibeWindow; project license applies. Local stateful MCP HTTP/SSE fixture.
import http.server
import json
import queue
import sys
import uuid

root, mode = sys.argv[1:3]
sessions = {}

def reply(message, identity):
    method = message.get("method")
    if "id" not in message:
        return None
    if method == "initialize":
        with open(root + "/starts", "a") as output:
            output.write(identity + "\n")
        result = {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "serverInfo": {"name": "http-fixture", "version": "1"}}
    elif method == "tools/list":
        result = {"tools": [{"name": "pid", "description": identity, "inputSchema": {"type": "object"}}]}
    else:
        result = {"content": [{"type": "text", "text": identity}]}
    return {"jsonrpc": "2.0", "id": message["id"], "result": result}

class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass
    def do_GET(self):
        if mode != "sse":
            self.send_error(405)
            return
        identity = str(uuid.uuid4())
        channel = queue.Queue()
        sessions[identity] = channel
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        try:
            self.wfile.write(("event: endpoint\ndata: /message?sessionId=" + identity + "\n\n").encode())
            self.wfile.flush()
            while True:
                try:
                    value = channel.get(timeout=0.1)
                    data = "event: message\ndata: " + json.dumps(value) + "\n\n"
                except queue.Empty:
                    data = ": alive\n\n"
                self.wfile.write(data.encode())
                self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            sessions.pop(identity, None)
    def do_POST(self):
        message = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        if mode == "sse":
            identity = self.path.split("sessionId=")[-1]
            value = reply(message, identity)
            if value:
                sessions[identity].put(value)
            self.send_response(202)
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        identity = self.headers.get("Mcp-Session-Id")
        if message.get("method") == "initialize":
            identity = str(uuid.uuid4())
            sessions[identity] = True
        elif identity not in sessions:
            self.send_error(404)
            return
        value = reply(message, identity)
        data = json.dumps(value).encode() if value is not None else b""
        self.send_response(200 if data else 202)
        self.send_header("Content-Type", "application/json")
        self.send_header("Mcp-Session-Id", identity)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
    def do_DELETE(self):
        sessions.pop(self.headers.get("Mcp-Session-Id"), None)
        self.send_response(204)
        self.end_headers()

server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
server.daemon_threads = True
with open(root + "/port", "w") as output:
    output.write(str(server.server_port))
server.serve_forever()
