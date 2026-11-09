"""Native MCP sandbox checks using a trusted TLS certificate for 127.0.0.1.

Set VW_WEBVIEW_HELPER, VW_MCP_TEST_CERT and VW_MCP_TEST_KEY. The desktop
session must trust the fixture CA through its normal certificate store.
"""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from queue import Empty, Queue
import ssl
import subprocess
from threading import Thread
import time
import unittest


class McpAppNetwork(unittest.TestCase):
    def setUp(self):
        self.helper = os.environ.get("VW_WEBVIEW_HELPER")
        self.cert = os.environ.get("VW_MCP_TEST_CERT")
        self.key = os.environ.get("VW_MCP_TEST_KEY")
        if not self.helper:
            self.skipTest("Set VW_WEBVIEW_HELPER to use a native desktop WebView.")
        self.requests = Queue()
        self.messages = Queue()
        self.diagnostics = []

    def https(self, routes):
        if not all((self.cert, self.key)):
            self.skipTest("Set VW_MCP_TEST_CERT and VW_MCP_TEST_KEY for trusted loopback TLS.")
        requests = self.requests

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                requests.put((self.server.server_port, self.path, self.headers.get("Origin")))
                status, headers, body = routes.get(self.path, (404, {}, ""))
                self.send_response(status)
                for key, value in headers.items():
                    self.send_header(key, value)
                self.end_headers()
                self.wfile.write(body.encode())

        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(self.cert, self.key)
        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.addCleanup(server.server_close)
        server.socket = context.wrap_socket(server.socket, server_side=True)
        thread = Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(thread.join)
        self.addCleanup(server.shutdown)
        return f"https://127.0.0.1:{server.server_port}", server.server_port

    def launch(self, html, csp):
        process = subprocess.Popen(
            [self.helper, "--mcp-app-stdin"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            encoding="utf-8",
        )

        def read_messages():
            for line in process.stdout:
                if line.startswith("mcp-app "):
                    self.messages.put(json.loads(line[len("mcp-app "):]))

        def read_diagnostics():
            self.diagnostics.extend(line.rstrip() for line in process.stderr)

        readers = [Thread(target=read_messages), Thread(target=read_diagnostics)]
        for thread in readers:
            thread.start()
        self.addCleanup(self.stop, process, readers)
        self.send(process, {"html": html, "csp": csp, "nonce": "a5" * 16})
        return process

    @staticmethod
    def send(process, message):
        process.stdin.write(json.dumps(message) + "\n")
        process.stdin.flush()

    @staticmethod
    def stop(process, readers):
        if process.poll() is None:
            try:
                process.stdin.write("close\n")
                process.stdin.flush()
                process.wait(timeout=4)
            except (BrokenPipeError, subprocess.TimeoutExpired):
                process.kill()
                process.wait()
        process.stdin.close()
        for reader in readers:
            reader.join(timeout=4)
        process.stdout.close()
        process.stderr.close()

    def message(self, expected):
        try:
            envelope = self.messages.get(timeout=10)
        except Empty:
            self.fail(f"Missing {expected}; helper diagnostics: {self.diagnostics}")
        self.assertEqual(envelope["origin"], "null")
        message = envelope["message"]
        self.assertEqual(message["method"], expected)
        return message

    def app(self, script):
        return """<!doctype html><meta charset="utf-8"><script>
        const rpc = (method, id, params) =>
            parent.postMessage({jsonrpc: '2.0', id, method, params}, '*');
        addEventListener('message', e => {
            if (e.source === parent && e.data.id === 1) {
                parent.postMessage({jsonrpc: '2.0', method: 'ui/notifications/initialized'}, '*');
                run();
            }
        });
        rpc('ui/initialize', 1, {});
        async function run() {
        """ + script + """
        }
        </script>"""

    def handshake(self, process):
        self.assertEqual(self.message("ui/initialize")["id"], 1)
        self.send(process, {"jsonrpc": "2.0", "id": 1, "result": {}})
        self.message("ui/notifications/initialized")

    def request_paths(self):
        paths = set()
        while not self.requests.empty():
            paths.add(self.requests.get_nowait())
        return paths

    def test_original_document_handshake_and_message_limit(self):
        page = self.app("""
            rpc('tools/call', 99, {body: 'x'.repeat(1048576)});
            rpc('tools/call', 2, {opaque: self.origin === 'null'});
        """)
        self.handshake(self.launch(page, {}))
        report = self.message("tools/call")
        self.assertEqual((report["id"], report["params"]), (2, {"opaque": True}))

    def test_declared_connect_and_resource_load(self):
        cdn, cdn_port = self.https({
            "/resource": (200, {"Content-Type": "text/javascript"}, "window.loadedAsset = 42;"),
        })
        api, api_port = self.https({
            "/api": (200, {"Access-Control-Allow-Origin": "null"}, "api response"),
            "/cors": (200, {"Access-Control-Allow-Origin": "https://elsewhere.test"}, "denied"),
        })
        page = self.app(f"""
            const script = document.createElement('script');
            script.src = '{cdn}/resource';
            const resource = new Promise(resolve => {{
                script.onload = () => resolve(window.loadedAsset);
                script.onerror = () => resolve('load error');
            }});
            document.head.append(script);
            const api = await fetch('{api}/api').then(r => r.text(), () => 'fetch error');
            const cors = await fetch('{api}/cors').then(() => 'unexpected success', e => e.name);
            rpc('tools/call', 2, {{resource: await resource, api, cors}});
        """)
        process = self.launch(page, {"resourceDomains": [cdn], "connectDomains": [api]})
        self.handshake(process)
        self.assertEqual(
            self.message("tools/call")["params"],
            {"resource": 42, "api": "api response", "cors": "TypeError"},
        )
        self.assertEqual(self.request_paths(), {
            (cdn_port, "/resource", None), (api_port, "/api", "null"),
            (api_port, "/cors", "null"),
        })

    def test_declared_frame_cannot_forge_rpc_or_escape_policy(self):
        denied, denied_port = self.https({})
        attack = """<script>
            const request = {jsonrpc:'2.0', id:99, method:'tools/call', params:{name:'forged'}};
            top.postMessage(request, '*');
            if (window.ipc) window.ipc.postMessage(JSON.stringify({nonce:'forged',message:request}));
            addEventListener('message', () => fetch('/received-host-data'));
            parent.postMessage('frame-loaded', '*');
        </script>"""
        frame, frame_port = self.https({
            "/frame": (200, {"Content-Type": "text/html"}, attack),
            "/replacement": (200, {"Content-Type": "text/html"}, attack),
            "/redirect": (302, {"Location": denied + "/redirected-frame"}, ""),
        })
        api, api_port = self.https({
            "/redirect": (302, {"Location": denied + "/redirected-api",
                                "Access-Control-Allow-Origin": "null"}, ""),
        })
        page = self.app(f"""
            const violations = new Set();
            addEventListener('securitypolicyviolation', e => violations.add(e.effectiveDirective));
            const iframe = document.createElement('iframe');
            const frame = new Promise(resolve => {{
                addEventListener('message', e => {{
                    if (e.source === iframe.contentWindow && e.data === 'frame-loaded') resolve(true);
                }});
                setTimeout(() => resolve(false), 3000);
            }});
            iframe.src = '{frame}/frame';
            document.body.append(iframe);
            for (const src of ['{denied}/direct-frame', '{frame}/redirect']) {{
                const frame = document.createElement('iframe');
                frame.src = src;
                document.body.append(frame);
            }}
            const fetches = await Promise.all(
                ['{denied}/direct-api?private=sample', '{api}/redirect', '{frame}/frame-as-api']
                    .map(url => fetch(url).then(() => 'allowed', e => e.name)));
            const image = new Image();
            image.src = '{api}/api-as-image';
            const loaded = await frame;
            await new Promise(resolve => setTimeout(resolve, 250));
            rpc('tools/call', 2, {{frame: loaded, fetches, violations: [...violations].sort()}});
            setTimeout(() => location.replace('{frame}/replacement'), 100);
        """)
        process = self.launch(page, {"frameDomains": [frame], "connectDomains": [api]})
        self.handshake(process)
        report = self.message("tools/call")
        self.assertEqual(report["id"], 2)
        self.assertEqual(report["params"], {
            "frame": True, "fetches": ["TypeError"] * 3,
            "violations": ["connect-src", "frame-src", "img-src"],
        })
        # A host delivery after attempted navigation must not reach the remote document.
        time.sleep(0.5)
        self.send(process, {"jsonrpc": "2.0", "method": "host-test"})
        time.sleep(0.4)
        self.assertTrue(self.messages.empty(), "Remote document emitted RPC")
        paths = self.request_paths()
        self.assertIn((frame_port, "/frame", None), paths)
        self.assertIn((frame_port, "/redirect", None), paths)
        self.assertIn((api_port, "/redirect", "null"), paths)
        self.assertFalse(any(port == denied_port for port, _, _ in paths), paths)
        self.assertFalse(any(path in {"/received-host-data", "/replacement", "/frame-as-api",
                                      "/api-as-image"} for _, path, _ in paths), paths)
        self.assertTrue(any("connect-src" in line for line in self.diagnostics), self.diagnostics)
        self.assertFalse(any("sample" in line for line in self.diagnostics), self.diagnostics)

    def test_invalid_policy_is_reported_without_echoing_input(self):
        process = self.launch("", {"connectDomains": ["https://example.test/?token=sample"]})
        process.wait(timeout=5)
        self.assertTrue(any("origin" in line for line in self.diagnostics), self.diagnostics)
        self.assertFalse(any("sample" in line for line in self.diagnostics), self.diagnostics)
        self.assertTrue(self.messages.empty())
