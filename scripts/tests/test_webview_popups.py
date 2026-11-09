"""Exercise native popup requests against a local HTTP server in a desktop session."""

import hashlib
import os
from pathlib import Path
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from queue import Empty, Queue
import subprocess
import sys
import tempfile
from threading import Thread
import time
import unittest
from uuid import uuid4


POST_BODY = b"message=a%2Bb+%26+caf%C3%A9&sequence=42"
DOWNLOAD_BODY = b"native popup download\x00\xff"


class WebViewPopups(unittest.TestCase):
    def setUp(self):
        self.helper = os.environ.get("VW_WEBVIEW_HELPER")
        if not self.helper:
            self.skipTest("Set VW_WEBVIEW_HELPER to run with a real desktop WebView.")
        self.linux = sys.platform.startswith("linux")
        self.window_title = f"shiju-popup-test-{uuid4().hex}"
        self.requests = Queue()
        self.routes = {}
        self.download_name = f"shiju-popup-{uuid4().hex}.bin"
        self.download_dir = Path.home() / "Downloads"
        if os.name != "nt":
            profile = tempfile.TemporaryDirectory()
            self.addCleanup(profile.cleanup)
            home = Path(profile.name)
            self.download_dir = home / "Downloads"
            self.download_dir.mkdir()
            config = home / ".config"
            config.mkdir()
            (config / "user-dirs.dirs").write_text('XDG_DOWNLOAD_DIR="$HOME/Downloads"\n')
            self.env = dict(os.environ, HOME=str(home), XDG_CONFIG_HOME=str(config),
                            XDG_DATA_HOME=str(home / ".local/share"))
        else:
            self.env = dict(os.environ)
        self.download_path = self.download_dir / self.download_name
        self.addCleanup(self.download_path.unlink, missing_ok=True)

        requests = self.requests
        routes = self.routes

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                self.respond()

            def do_POST(self):
                self.respond()

            def respond(self):
                body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
                requests.put((self.path, self.command, self.headers, body))
                content, headers = routes.get(self.path, (b"", {}))
                self.send_response(200)
                self.send_header("Content-Length", str(len(content)))
                for name, value in headers.items():
                    self.send_header(name, value)
                self.end_headers()
                self.wfile.write(content)

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.addCleanup(server.server_close)
        thread = Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(thread.join)
        self.addCleanup(server.shutdown)
        self.origin = f"http://127.0.0.1:{server.server_port}"

    def page(self, path, html, **headers):
        self.routes[path] = (
            ("<!doctype html><meta charset=utf-8>" + html).encode(),
            {"Content-Type": "text/html; charset=utf-8", **headers},
        )

    def action_page(self, path, html, action, **headers):
        if self.linux:
            # WebKitGTK expects a real user gesture before opening a popup.
            html += ('<button id="run" style="position:fixed;inset:0;width:100%;height:100%">'
                     'Run</button><script>'
                     f'document.querySelector("#run").onclick = () => {{ {action} }};'
                     f'fetch("{path}-ready");</script>')
        else:
            html += f"<script>{action}</script>"
        self.page(path, html, **headers)

    def click(self, title):
        window = subprocess.check_output(
            ["xdotool", "search", "--onlyvisible", "--name", f"^{title}$"],
            text=True, timeout=5,
        ).splitlines()[-1]
        subprocess.run(
            ["xdotool", "windowraise", window, "mousemove", "--window", window, "100", "100",
             "click", "1"],
            check=True, timeout=5,
        )

    def launch(self, path):
        process = subprocess.Popen(
            [self.helper, self.origin + path, "--mode=embedded", f"--title={self.window_title}"],
            stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL,
            env=self.env,
            text=True,
        )
        self.addCleanup(self.stop, process)
        if self.linux:
            self.request(path + "-ready")
            self.click(self.window_title)
        return process

    @staticmethod
    def stop(process):
        with process.stdin:
            if process.poll() is None:
                try:
                    process.stdin.write("close\n")
                    process.stdin.flush()
                    process.wait(timeout=4)
                except (BrokenPipeError, subprocess.TimeoutExpired):
                    process.kill()
                    process.wait()

    def request(self, path):
        deadline = time.monotonic() + 12
        while time.monotonic() < deadline:
            try:
                received_path, method, headers, body = self.requests.get(
                    timeout=max(0, deadline - time.monotonic())
                )
            except Empty:
                break
            if received_path == path:
                return method, headers, body
        self.fail(f"WebView did not request {path}")

    def test_target_blank_preserves_post_body(self):
        self.action_page(
            "/start",
            """<form method="post" action="/submit" target="_blank">
                <input name="message" value="a+b &amp; café">
                <input name="sequence" value="42">
            </form>""",
            "document.forms[0].submit()",
            **{"Set-Cookie": "popup_session=synthetic; Path=/; SameSite=Lax"},
        )
        self.page("/submit", "<script>fetch('/post-response')</script>")
        self.launch("/start")
        method, headers, body = self.request("/submit")
        print(f"Popup form: {method}, {len(body)} bytes, SHA-256 {hashlib.sha256(body).hexdigest()}")
        self.assertEqual((method, body), ("POST", POST_BODY))
        self.assertEqual(headers.get_content_type(), "application/x-www-form-urlencoded")
        self.assertIn("popup_session=synthetic", headers.get("Cookie", ""))
        self.request("/post-response")

    def test_blank_popup_can_navigate_read_write_opener_and_close(self):
        self.action_page(
            "/start",
            "",
            """window.parentValue = 'parent';
                const child = window.open('about:blank');
                if (child) {
                    child.initialValue = 'child';
                    if (child.initialValue === 'child') child.location = '/popup';
                    const timer = setInterval(() => {
                        if (child.closed && window.childValue === 'written') {
                            clearInterval(timer);
                            location.href = '/parent-alive';
                        }
                    }, 30);
                }""",
        )
        self.page(
            "/popup",
            """<script>
                if (opener.parentValue === 'parent') {
                    opener.childValue = 'written';
                    window.close();
                }
            </script>""",
        )
        process = self.launch("/start")
        self.request("/popup")
        self.request("/parent-alive")
        self.assertIsNone(process.poll())

    def test_same_page_link_navigates(self):
        self.action_page("/start", "<a href='/same-page'>Link</a>", "document.links[0].click()")
        self.launch("/start")
        self.assertEqual(self.request("/same-page")[0], "GET")

    def assert_download(self, payload):
        deadline = time.monotonic() + 12
        while time.monotonic() < deadline:
            if self.download_path.exists() and self.download_path.read_bytes() == payload:
                return
            time.sleep(0.05)
        self.fail("WebView did not save the expected download bytes")

    def test_popup_attachment_downloads(self):
        self.action_page(
            "/start", "<form method=post action=/export target=_blank></form>",
            "document.forms[0].submit()",
        )
        # Renderable MIME type exercises Content-Disposition handling in WebKit.
        self.routes["/export"] = (
            DOWNLOAD_BODY,
            {
                "Content-Type": "text/plain",
                "Content-Disposition": f'attachment; filename="{self.download_name}"',
            },
        )
        self.launch("/start")
        self.assertEqual(self.request("/export")[0], "POST")
        self.assert_download(DOWNLOAD_BODY)

    def test_popup_blob_downloads(self):
        self.check_generated_download("URL.createObjectURL(new Blob(['generated']))")

    def test_popup_data_downloads(self):
        self.check_generated_download("'data:text/plain,generated'")

    def check_generated_download(self, url_expression):
        self.action_page(
            "/start", "<a href=/popup target=_blank>Open</a>", "document.links[0].click()",
        )
        self.action_page(
            "/popup",
            f'<a download="{self.download_name}">Download</a>',
            f"""const link = document.querySelector('a');
                link.href = {url_expression};
                link.click();""",
        )
        self.launch("/start")
        if self.linux:
            self.request("/popup-ready")
            self.click("Vibe Window WebView")
        else:
            self.request("/popup")
        self.assert_download(b"generated")
