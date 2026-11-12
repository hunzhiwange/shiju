"""Loopback checks against voice-window built in the caller's held Cargo slot."""
import argparse
import http.server
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import unittest

BINARY = None


class Gateway(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.server.requests.append((self.path, dict(self.headers)))
        body = b'{"dictation_enabled":false,"realtime_enabled":false}'
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_):
        pass


class LiveBootstrapTests(unittest.TestCase):
    def setUp(self):
        self.storage = tempfile.TemporaryDirectory(prefix="voice-bootstrap-test-")
        self.addCleanup(self.storage.cleanup)
        self.root = Path(self.storage.name)
        self.gateway = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Gateway)
        self.gateway.requests = []
        thread = threading.Thread(target=self.gateway.serve_forever, daemon=True)
        thread.start()
        def stop():
            self.gateway.shutdown()
            self.gateway.server_close()
            thread.join()
        self.addCleanup(stop)
        self.bootstrap = {
            "active_server_id": "chosen",
            "servers": [
                {"id": "decoy", "host": "127.0.0.1", "port": 1},
                {"id": "chosen", "host": "127.0.0.1", "port": self.gateway.server_port,
                 "skey": "synthetic-acceptance-key", "platform_id": "10001",
                 "company_id": "100000001"},
            ],
        }

    def run_live(self, payload):
        path = self.root / "operator-bootstrap.json"
        path.write_bytes(payload)
        env = dict(os.environ, VOICE_ACCEPTANCE_GATEWAY_BOOTSTRAP=str(path),
                   VOICE_ACCEPTANCE_SESSION="isolated-bootstrap-check")
        result = subprocess.run([str(BINARY), "--scenario", "live"], env=env,
                                capture_output=True, text=True, timeout=20)
        self.assertEqual(path.read_bytes(), payload, "operator bootstrap was modified")
        return result

    def test_live_reads_selected_gateway_before_requesting_audio(self):
        result = self.run_live(json.dumps(self.bootstrap).encode())
        self.assertEqual(result.returncode, 2)
        self.assertEqual(len(self.gateway.requests), 1,
                         "live ignored the explicitly selected gateway bootstrap: " + result.stderr)
        path, headers = self.gateway.requests[0]
        headers = {key.lower(): value for key, value in headers.items()}
        self.assertEqual(path, "/api/voice/config")
        self.assertEqual(headers.get("authorization"), "Bearer synthetic-acceptance-key")
        self.assertEqual(headers.get("x-platform-id"), "10001")
        self.assertEqual(headers.get("x-company-id"), "100000001")
        self.assertIn("live requires configured dictation and realtime services", result.stderr)
        # Disabled synthetic services stop preflight before permissions, capture or playback.
        self.assertNotIn("VOICE_EVIDENCE", result.stderr)

    def test_invalid_live_bootstrap_fails_before_gateway_request(self):
        result = self.run_live(b"{invalid")
        self.assertEqual(result.returncode, 2)
        self.assertIn("invalid ShiJu gateway bootstrap", result.stderr)
        self.assertEqual(self.gateway.requests, [])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    args = parser.parse_args()
    target = os.environ.get("CARGO_TARGET_DIR")
    if not target or not args.binary.resolve().is_relative_to(Path(target).resolve()):
        parser.error("binary must belong to the caller's held CARGO_TARGET_DIR")
    BINARY = args.binary.resolve()
    unittest.main(argv=[sys.argv[0], "-v"])
