"""Native getUserMedia probe. Use a desktop with no other media capture running.

Set VW_WEBVIEW_HELPER to the built helper (or the one inside ShiJu.app).
Set VW_WEBVIEW_MEDIA_EXPECT to allow, deny, or missing for this run. Respond to
the native prompts accordingly. In missing mode, disconnect both input devices
first. Only track kinds/states and DOMException names are sent to the server.
Linux requires X11 and xdotool for the popup-opening user gesture.
"""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
from queue import Empty, Queue
import subprocess
import sys
import tempfile
from threading import Thread
import time
import unittest
from uuid import uuid4


MEDIA_PAGE = r"""
<h1>Camera and microphone test</h1>
<pre id="result"></pre>
<script>
async function testMedia() {
    for (const [name, constraints] of [
        ['camera', {video: true}],
        ['microphone', {audio: true}],
        ['combined', {audio: true, video: true}],
    ]) {
        let stream;
        let result;
        try {
            stream = await navigator.mediaDevices.getUserMedia(constraints);
            result = {tracks: stream.getTracks().map(track => [track.kind, track.readyState])};
        } catch (error) {
            result = {error: error.name};
        }
        document.querySelector('#result').textContent += name + ': ' + JSON.stringify(result) + '\n';
        await fetch('/result/' + name, {method: 'POST', body: JSON.stringify(result)});
        // Leave the final successful stream active until its owning window closes.
        if (stream && name !== 'combined') stream.getTracks().forEach(track => track.stop());
        else if (stream) window.testStream = stream;
    }
    if (window.opener) window.close();
}
testMedia();
</script>
"""
POPUP_PAGE = """
<button style="position:fixed;inset:0;width:100%;height:100%">Open media popup</button>
<script>
function openPopup() {
    const popup = window.open('/media');
    if (popup) {
        const timer = setInterval(() => {
            if (popup.closed) {
                clearInterval(timer);
                fetch('/popup-closed', {method: 'POST'});
            }
        }, 100);
    }
}
document.querySelector('button').onclick = openPopup;
POPUP_START
</script>
"""


class WebViewMediaTests(unittest.TestCase):
    def setUp(self):
        self.helper = os.environ.get("VW_WEBVIEW_HELPER")
        if not self.helper:
            self.skipTest("Set VW_WEBVIEW_HELPER to run with a desktop WebView.")
        self.expect = os.environ.get("VW_WEBVIEW_MEDIA_EXPECT", "allow")
        self.assertIn(self.expect, ("allow", "deny", "missing"))
        self.events = Queue()
        events = self.events

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                popup_start = (
                    "fetch('/ready', {method: 'POST'});"
                    if sys.platform.startswith("linux") else "openPopup();"
                )
                pages = {"/media": MEDIA_PAGE,
                         "/popup": POPUP_PAGE.replace("POPUP_START", popup_start)}
                if self.path not in pages:
                    self.send_error(404)
                    return
                body = ("<!doctype html><meta charset=utf-8>" + pages[self.path]).encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def do_POST(self):
                size = int(self.headers.get("Content-Length", "0"))
                body = self.rfile.read(size)
                self.send_response(204)
                self.end_headers()
                events.put((self.path, json.loads(body) if body else None))

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.addCleanup(server.server_close)
        thread = Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(thread.join)
        self.addCleanup(server.shutdown)
        self.origin = f"http://127.0.0.1:{server.server_port}"
        profile = tempfile.TemporaryDirectory()
        self.addCleanup(profile.cleanup)
        self.env = dict(os.environ, HOME=profile.name, XDG_DATA_HOME=profile.name,
                        LOCALAPPDATA=profile.name)
        self.title = f"shiju-media-test-{uuid4().hex}"

    def launch(self, path):
        helper = str(Path(self.helper).resolve(strict=True))
        process = subprocess.Popen(
            [helper, self.origin + path, "--mode=window", f"--title={self.title}"],
            stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, env=self.env, text=True,
        )
        self.addCleanup(self.stop, process)
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
                    process.wait(timeout=5)
                except (BrokenPipeError, subprocess.TimeoutExpired):
                    process.kill()
                    process.wait()

    def event(self, expected):
        try:
            path, result = self.events.get(timeout=60)
        except Empty:
            self.fail(f"No WebView result for {expected}. Check native permission prompts.")
        self.assertEqual(path, expected)
        return result

    def open_popup(self):
        if not sys.platform.startswith("linux"):
            return
        self.event("/ready")
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            search = subprocess.run(
                ["xdotool", "search", "--onlyvisible", "--name", f"^{self.title}$"],
                text=True, capture_output=True, timeout=2,
            )
            if search.returncode == 0:
                window = search.stdout.splitlines()[-1]
                subprocess.run(
                    ["xdotool", "windowraise", window, "mousemove", "--window", window,
                     "100", "100", "click", "1"], check=True, timeout=5,
                )
                return
            time.sleep(0.05)
        self.fail("Media test window did not become visible")

    def assert_results(self):
        for name, kinds in (("camera", ["video"]), ("microphone", ["audio"]),
                            ("combined", ["audio", "video"])):
            result = self.event("/result/" + name)
            print(f"{name}: {result}", flush=True)
            with self.subTest(media=name):
                if self.expect == "allow":
                    self.assertEqual(sorted(result.get("tracks", [])),
                                     [[kind, "live"] for kind in kinds])
                elif self.expect == "deny":
                    self.assertEqual(result, {"error": "NotAllowedError"})
                else:
                    missing = ["NotFoundError"]
                    if sys.platform.startswith("linux"):
                        # Some WebKitGTK versions report missing devices as unsatisfied
                        # default capture constraints, even for {audio/video: true}.
                        missing.append("OverconstrainedError")
                    self.assertIn(result.get("error"), missing)

    def test_main_view(self):
        process = self.launch("/media")
        self.assert_results()
        # Require a clean exit with the final stream still active.
        process.stdin.write("close\n")
        process.stdin.flush()
        self.assertEqual(process.wait(timeout=8), 0)

    def test_related_popup(self):
        process = self.launch("/popup")
        self.open_popup()
        self.assert_results()
        self.event("/popup-closed")
        self.assertIsNone(process.poll())


if __name__ == "__main__":
    unittest.main()
