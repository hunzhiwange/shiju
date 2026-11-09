"""Loopback Messages API for the fixed Claude desktop approval acceptance test."""

import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from acp_skill_mount_claude import messages_wire


def serve(workspace):
    tools = [
        ("Write", {"file_path": str(workspace / name), "content": "must stay absent"})
        for name in ("native.txt", "organization.txt", ".protected", "../outside.txt")
    ]
    tools.extend([
        ("Bash", {"command": "touch command.txt", "description": "Create the command marker"}),
        ("Bash", {"command": "rm keep.txt", "description": "Remove the deletion marker"}),
    ])
    calls = iter(tools)

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_POST(self):
            if self.headers.get("x-api-key") != "shiju-offline-fixture":
                self.send_error(401)
                return
            length = int(self.headers["Content-Length"])
            if not 0 < length < 2_000_000:
                self.send_error(400)
                return
            request = json.loads(self.rfile.read(length))
            route = self.path.split("?", 1)[0]
            if route == "/v1/messages/count_tokens":
                payload, content_type = b'{"input_tokens":1}', "application/json"
            elif route == "/v1/messages":
                tool = next(calls, None)
                message = {"content": "FALLBACK_FIXTURE_DONE"}
                if tool:
                    name, arguments = tool
                    message = {"tool_calls": [{
                        "id": f"toolu_{self.server.tool_index}",
                        "function": {"name": name, "arguments": json.dumps(arguments)},
                    }]}
                    self.server.tool_index += 1
                response, wire = messages_wire(message, request["model"])
                if request.get("stream"):
                    payload, content_type = wire, "text/event-stream"
                else:
                    payload, content_type = json.dumps(response).encode(), "application/json"
            else:
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

    with ThreadingHTTPServer(("127.0.0.1", 0), Handler) as server:
        server.tool_index = 0
        print(f"http://127.0.0.1:{server.server_port}", flush=True)
        server.serve_forever()


if __name__ == "__main__":
    serve(Path(sys.argv[1]))
