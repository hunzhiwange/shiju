"""Exercise native popup requests against a local HTTP server in a desktop session.

The Linux protocol case uses xdotool for input and xprop to check dialog ownership.
"""

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

    def launch(self, path, mode="embedded"):
        process = subprocess.Popen(
            [self.helper, self.origin + path, f"--mode={mode}", f"--title={self.window_title}"],
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
        if process.stdin.closed:
            return
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

    def visible_window(self, title):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            search = subprocess.run(
                ["xdotool", "search", "--onlyvisible", "--name", f"^{title}$"],
                text=True, capture_output=True, timeout=2,
            )
            if search.returncode == 0:
                return search.stdout.splitlines()[-1]
            time.sleep(0.05)
        self.fail(f"Window did not appear: {title}")

    @staticmethod
    def send_keys(window, *keys):
        subprocess.run(["xdotool", "windowfocus", window, "key", *keys], check=True, timeout=5)

    @unittest.skipUnless(sys.platform.startswith("linux"), "Requires GTK, GIO and X11 input")
    def test_external_protocol_requires_confirmation_and_uses_registered_handler(self):
        scheme = f"shiju-protocol-{uuid4().hex}"
        unknown = f"shiju-missing-{uuid4().hex}"
        target = f"{scheme}://local/synthetic?number=42"
        home = Path(self.env["HOME"])
        capture = home / "received.txt"
        receiver = home / "receive.py"
        receiver.write_text(
            "import sys\nwith open(sys.argv[1], 'a') as out:\n    print(sys.argv[2], file=out)\n"
        )
        applications = Path(self.env["XDG_DATA_HOME"]) / "applications"
        applications.mkdir(parents=True)
        desktop = "shiju-protocol-test.desktop"
        (applications / desktop).write_text(
            f"[Desktop Entry]\nName=ShiJu protocol test\nType=Application\nNoDisplay=true\n"
            f"Exec=/usr/bin/python3 {receiver} {capture} %u\nMimeType=x-scheme-handler/{scheme};\n"
        )
        (Path(self.env["XDG_CONFIG_HOME"]) / "mimeapps.list").write_text(
            f"[Default Applications]\nx-scheme-handler/{scheme}={desktop};\n"
        )

        def link(path, href, extra=""):
            self.action_page(
                path, f"<a href='{href}' {extra}>Open</a>",
                "document.links[0].click(); document.links[0].click()",
            )

        def confirmation(parent_title):
            owner = self.visible_window(parent_title)
            dialog = self.visible_window("打开外部应用")
            transient = subprocess.check_output(["xprop", "-id", dialog, "WM_TRANSIENT_FOR"],
                                                text=True, timeout=5)
            self.assertEqual(int(transient.split()[-1], 16), int(owner))
            return dialog

        # Same-view links do not launch until the host receives approval.
        link("/cancel", target)
        process = self.launch("/cancel")
        dialog = confirmation(self.window_title)
        self.assertFalse(capture.exists())
        self.send_keys(dialog, "Escape")
        self.assertFalse(capture.exists())
        self.assertIsNone(process.poll())
        self.stop(process)

        # A target=_blank link remains associated with the opener. Two requests
        # during the same confirmation must launch the registered handler once.
        link("/new-window", target, "target='_blank'")
        process = self.launch("/new-window")
        dialog = confirmation(self.window_title)
        self.assertFalse(capture.exists())
        self.send_keys(dialog, "Tab", "Return")
        deadline = time.monotonic() + 5
        while not capture.exists() and time.monotonic() < deadline:
            time.sleep(0.05)
        self.assertTrue(capture.exists(), "The registered app did not receive the URL")
        time.sleep(0.3)
        self.assertEqual(capture.read_text().splitlines(), [target])
        self.stop(process)

        # Enter and the globe button on the trusted toolbar share the same gate.
        self.action_page("/toolbar", "", "")
        process = self.launch("/toolbar", mode="window")
        window = self.visible_window(self.window_title)
        for keys in [("Return",), ("Tab", "Tab", "Return")]:
            subprocess.run(
                ["xdotool", "windowfocus", window, "mousemove", "--window", window, "180", "20",
                 "click", "1", "key", "ctrl+a", "type", "--clearmodifiers", target],
                check=True, timeout=5,
            )
            subprocess.run(["xdotool", "key", *keys], check=True, timeout=5)
            dialog = confirmation(self.window_title)
            self.send_keys(dialog, "Escape")
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline and subprocess.run(
                ["xdotool", "search", "--onlyvisible", "--name", "^打开外部应用$"],
                stdout=subprocess.DEVNULL, timeout=2,
            ).returncode == 0:
                time.sleep(0.05)
            self.assertLess(time.monotonic(), deadline, "confirmation did not close")
        self.stop(process)

        # An already open popup owns its own dialog, including failure feedback.
        link("/opener", "/child", "target='_blank'")
        link("/child", f"{unknown}:synthetic")
        self.launch("/opener")
        self.request("/child-ready")
        self.click("Vibe Window WebView")
        dialog = confirmation("Vibe Window WebView")
        self.send_keys(dialog, "Tab", "Return")
        failure = self.visible_window("无法打开外部应用")
        self.send_keys(failure, "Return")
        self.assertEqual(capture.read_text().splitlines(), [target])

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
