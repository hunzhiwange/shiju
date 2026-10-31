"""MIT; loopback Streamable HTTP elicitation with an actual reverse response POST."""
import http.server
import json
import pathlib
import sys
import threading

response_ready = threading.Event()
response = None
initialize = None

class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *_args):
        pass

    def reply(self, status, value=None):
        body = b"" if value is None else json.dumps(value).encode()
        self.send_response(status)
        if value is not None:
            self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
        self.wfile.flush()

    def do_GET(self):
        self.reply(405)

    def do_DELETE(self):
        self.reply(200)

    def do_POST(self):
        global response, initialize
        msg = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))))
        method = msg.get("method")
        if method == "initialize":
            initialize = msg["params"]
            self.reply(200, {"jsonrpc":"2.0", "id":msg["id"], "result":{
                "protocolVersion":"2025-06-18", "capabilities":{"tools":{}},
                "serverInfo":{"name":"form-http-fixture","version":"1"}}})
        elif method == "notifications/initialized":
            self.reply(202)
        elif method == "tools/list":
            self.reply(200, {"jsonrpc":"2.0", "id":msg["id"], "result":{"tools":[
                {"name":"form", "inputSchema":{"type":"object"}}]}})
        elif method == "tools/call":
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Connection", "close")
            self.end_headers()
            form = {"jsonrpc":"2.0", "id":"http-form", "method":"elicitation/create", "params":{
                "mode":"form", "message":"HTTP 表单", "requestedSchema":{
                    "type":"object", "properties":{"confirmed":{"type":"boolean"}}, "required":["confirmed"]}}}
            self.wfile.write(("event: message\ndata: "+json.dumps(form)+"\n\n").encode())
            self.wfile.flush()
            if response_ready.wait(5):
                result = {"jsonrpc":"2.0", "id":msg["id"], "result":{"content":[],
                    "structuredContent":{"form_reply":response,"initialize":initialize}}}
                self.wfile.write(("event: message\ndata: "+json.dumps(result)+"\n\n").encode())
                self.wfile.flush()
            self.close_connection = True
        elif msg.get("id") == "http-form":
            response = msg
            self.reply(202)
            response_ready.set()
        else:
            self.reply(400)

server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
pathlib.Path(sys.argv[1]).write_text(str(server.server_port))
server.serve_forever()
