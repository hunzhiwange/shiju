"""Original MIT-licensed loopback MCP server requiring the negotiated HTTP header."""
import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

version, port_path, log_path, mode = sys.argv[1:]
initialized = None


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def record(self, method):
        with open(log_path, "a") as output:
            output.write(json.dumps({"method": method,
                "protocol": self.headers.get("MCP-Protocol-Version")}) + "\n")

    def empty(self, status):
        self.send_response(status)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_POST(self):
        global initialized
        message = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
        method = message.get("method")
        self.record(method)
        if method != "initialize" and self.headers.get("MCP-Protocol-Version") != version:
            return self.empty(400)
        if method == "initialize":
            initialized = message["params"]
            result = {"protocolVersion": version, "capabilities": {"tools": {}},
                "serverInfo": {"name": "fixture", "version": "1"}}
        elif method == "tools/call":
            result = {"content": [], "structuredContent": {"initialize": initialized}}
        elif method == "tools/list":
            result = {"tools": []}
        else:
            return self.empty(202)
        payload = json.dumps({"jsonrpc": "2.0", "id": message["id"], "result": result})
        if mode == "sse":
            payload = "event: message\ndata: " + payload + "\n\n"
        encoded = payload.encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream" if mode == "sse" else "application/json")
        self.send_header("Mcp-Session-Id", "fixture-session")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def do_GET(self):
        self.record("GET")
        self.empty(405 if self.headers.get("MCP-Protocol-Version") == version else 400)

    def do_DELETE(self):
        self.record("DELETE")
        self.empty(204 if self.headers.get("MCP-Protocol-Version") == version else 400)


server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
with open(port_path, "w") as output:
    output.write(str(server.server_port))
server.serve_forever()
