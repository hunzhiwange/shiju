"""Exercise native form navigation in a desktop WebView.

Set VW_WEBVIEW_HELPER and provide a locally trusted loopback TLS fixture
via VW_MCP_TEST_CERT and VW_MCP_TEST_KEY. Run on an isolated X11 display
with xdotool to exercise the real confirmation dialog.
"""

from email import policy
from email.parser import BytesParser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import os
from queue import Empty
import shutil
import ssl
import subprocess
from threading import Thread
import unittest

import test_mcp_app_network as network


class McpAppForms(unittest.TestCase):
    setUp = network.McpAppNetwork.setUp
    launch = network.McpAppNetwork.launch
    send = staticmethod(network.McpAppNetwork.send)
    stop = staticmethod(network.McpAppNetwork.stop)
    message = network.McpAppNetwork.message
    app = network.McpAppNetwork.app
    handshake = network.McpAppNetwork.handshake

    def https(self, routes):
        if not all((self.cert, self.key)):
            self.skipTest("Set VW_MCP_TEST_CERT and VW_MCP_TEST_KEY for trusted loopback TLS.")
        requests = self.requests

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def handle_form(self):
                body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
                requests.put((self.server.server_port, self.command, self.path,
                              self.headers.get("Content-Type"), body))
                status, headers, html = routes.get(self.path, (404, {}, ""))
                self.send_response(status)
                for name, value in headers.items():
                    self.send_header(name, value)
                self.end_headers()
                self.wfile.write(html.encode())

            do_GET = do_POST = handle_form

        tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        tls.load_cert_chain(self.cert, self.key)
        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.addCleanup(server.server_close)
        server.socket = tls.wrap_socket(server.socket, server_side=True)
        thread = Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(thread.join)
        self.addCleanup(server.shutdown)
        return f"https://127.0.0.1:{server.server_port}", server.server_port

    def form(self, url, *, target="_self", method="post", encoding="application/x-www-form-urlencoded"):
        return self.app(f"""
            const form = document.createElement('form');
            form.method = '{method}';
            form.action = '{url}';
            form.target = '{target}';
            form.enctype = '{encoding}';
            form.innerHTML = '<input name="fixture" value="x + 雪"><button name="submitter" value="go">Send</button>';
            document.body.append(form);
            form.requestSubmit(form.querySelector('button'));
        """)

    def confirmation_window(self):
        if not (os.getenv("DISPLAY") and shutil.which("xdotool")):
            self.skipTest("Use an isolated X11 display with xdotool for native consent.")
        try:
            return subprocess.run(
                ["xdotool", "search", "--sync", "--onlyvisible", "--name", "^提交 MCP 表单$"],
                check=True, capture_output=True, text=True, timeout=5,
            ).stdout.splitlines()[-1]
        except subprocess.TimeoutExpired:
            self.fail(f"No native form confirmation. Diagnostics: {self.diagnostics}")

    def answer(self, accept):
        window = self.confirmation_window()
        key = "Right Return" if accept else "Escape"
        subprocess.run(["xdotool", "windowfocus", "--sync", window, "key", *key.split()],
                       check=True, timeout=4)

    def request(self):
        try:
            return self.requests.get(timeout=6)
        except Empty:
            self.fail(f"No fixture request. Diagnostics: {self.diagnostics}")

    def test_declared_origin_keeps_bridge_available_until_submission(self):
        process = self.launch(
            self.app("rpc('tools/call', 3, {ready: true});"),
            {"connectDomains": ["https://fixture.example"]},
        )
        self.handshake(process)
        self.assertEqual(self.message("tools/call")["params"], {"ready": True})

    def test_approved_form_posts_without_granting_bridge_access(self):
        response = """<script>
          const forged = {jsonrpc:'2.0', id:99, method:'tools/call', params:{name:'forged'}};
          top.postMessage(forged, '*');
          window.ipc?.postMessage(JSON.stringify({nonce:'a5'.repeat(16),message:forged}));
          addEventListener('message', () => fetch('/leak'));
          fetch('/rendered');
        </script>"""
        api, port = self.https({"/submit": (200, {"Content-Type": "text/html"}, response)})
        process = self.launch(self.form(api + "/submit"), {"connectDomains": [api]})
        self.handshake(process)
        self.answer(True)
        self.assertEqual(self.request(), (
            port, "POST", "/submit", "application/x-www-form-urlencoded",
            b"fixture=x+%2B+%E9%9B%AA&submitter=go",
        ))
        self.assertEqual(self.request()[:3], (port, "GET", "/rendered"))
        self.send(process, {"jsonrpc": "2.0", "method": "host-test"})
        self.assert_no_request()
        self.assertTrue(self.messages.empty(), "Result document reached the MCP bridge")

    def test_native_get_preserves_query_encoding(self):
        api, port = self.https({})
        process = self.launch(self.form(api + "/submit", method="get"), {"connectDomains": [api]})
        self.handshake(process)
        self.answer(True)
        self.assertEqual(self.request(), (
            port, "GET", "/submit?fixture=x+%2B+%E9%9B%AA&submitter=go", None, b"",
        ))

    def test_native_multipart_preserves_encoding_and_fields(self):
        api, port = self.https({})
        process = self.launch(
            self.form(api + "/multipart", encoding="multipart/form-data"), {"connectDomains": [api]},
        )
        self.handshake(process)
        self.answer(True)
        actual_port, method, path, content_type, body = self.request()
        self.assertEqual((actual_port, method, path), (port, "POST", "/multipart"))
        self.assertTrue(content_type.startswith("multipart/form-data; boundary="), content_type)
        message = BytesParser(policy=policy.default).parsebytes(
            f"Content-Type: {content_type}\r\nMIME-Version: 1.0\r\n\r\n".encode() + body,
        )
        self.assertEqual([
            (part.get_param("name", header="Content-Disposition"), part.get_payload(decode=True))
            for part in message.iter_parts()
        ], [("fixture", "x + 雪".encode()), ("submitter", b"go")])

    def assert_no_request(self):
        with self.assertRaises(Empty, msg="Unexpected network request"):
            self.requests.get(timeout=0.7)

    def test_cancel_preserves_the_original_bridge_without_posting(self):
        api, _ = self.https({})
        page = self.form(api + "/cancel").replace(
            "const form", """addEventListener('message', event => {
                if (event.data.method === 'host-test') rpc('tools/call', 3, {stillLive: true});
            });
            const form""", 1,
        )
        process = self.launch(page, {"connectDomains": [api]})
        self.handshake(process)
        self.answer(False)
        self.assert_no_request()
        self.send(process, {"jsonrpc": "2.0", "method": "host-test"})
        self.assertEqual(self.message("tools/call")["params"], {"stillLive": True})
        self.assertTrue(any("submission cancelled" in line for line in self.diagnostics))

    def test_undeclared_origin_and_ancestor_or_popup_targets_are_denied(self):
        api, _ = self.https({})
        denied, _ = self.https({})
        page = self.app(f"""
            for (const [action, target] of [
                ['{denied}/denied', '_self'], ['{api}/top', '_top'],
                ['{api}/parent', '_parent'], ['{api}/blank', '_blank'],
                ['{api}/auxiliary', 'new-window'],
            ]) {{
                const form = document.createElement('form');
                Object.assign(form, {{action, target, method: 'post'}});
                document.body.append(form);
                form.submit();
                await new Promise(resolve => setTimeout(resolve, 120));
            }}
            rpc('tools/call', 3, {{completed: true}});
            location.assign('{api}/not-a-form');
        """)
        process = self.launch(page, {"connectDomains": [api]})
        self.handshake(process)
        self.assertEqual(self.message("tools/call")["params"], {"completed": True})
        self.assert_no_request()
        if os.getenv("DISPLAY") and shutil.which("xdotool"):
            dialog = subprocess.run(
                ["xdotool", "search", "--onlyvisible", "--name", "^提交 MCP 表单$"],
                capture_output=True, timeout=4,
            )
            self.assertNotEqual(dialog.returncode, 0, "Unsafe target reached confirmation")

    def test_cross_origin_redirect_cannot_replay_post_to_another_declared_origin(self):
        other, _ = self.https({})
        api, port = self.https({"/redirect": (307, {"Location": other + "/replay"}, "")})
        process = self.launch(self.form(api + "/redirect"), {"connectDomains": [api, other]})
        self.handshake(process)
        self.answer(True)
        self.assertEqual(self.request()[:3], (port, "POST", "/redirect"))
        self.assert_no_request()

    def test_closed_session_cannot_submit_a_pending_form(self):
        api, _ = self.https({})
        process = self.launch(self.form(api + "/stale"), {"connectDomains": [api]})
        self.handshake(process)
        self.confirmation_window()
        process.stdin.write("close\n")
        process.stdin.flush()
        self.assertEqual(process.wait(timeout=4), 0)
        self.assert_no_request()


if __name__ == "__main__":
    unittest.main()
