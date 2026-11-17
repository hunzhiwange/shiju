"""Real helper/socket tests; require an existing SHIJU_CONTROL_HELPER executable.

Never builds, requests TCC, captures the screen or injects input. Missing artifacts
are errors, not skipped acceptance. Physical acceptance is documented separately.
"""
import json
import os
from pathlib import Path
import socket
import stat
import struct
import subprocess
import sys
import tempfile
import unittest

MAX_FRAME = 1024 * 1024


def send_frame(channel, value):
    data = json.dumps(value).encode()
    if not 0 < len(data) <= MAX_FRAME:
        raise ValueError("invalid frame size")
    channel.sendall(struct.pack("!I", len(data)) + data)


def receive_frame(channel):
    def exact(size):
        data = bytearray()
        while len(data) < size:
            part = channel.recv(size - len(data))
            if not part:
                raise EOFError("helper closed channel")
            data.extend(part)
        return data
    size = struct.unpack("!I", exact(4))[0]
    if not 0 < size <= MAX_FRAME:
        raise ValueError("invalid frame size")
    return json.loads(exact(size))


class RuntimeIntegrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        configured = os.environ.get("SHIJU_CONTROL_HELPER")
        if not configured:
            raise RuntimeError("set SHIJU_CONTROL_HELPER to an existing helper; no automatic build")
        cls.helper = Path(configured).resolve(strict=True)
        if not cls.helper.is_file() or not os.access(cls.helper, os.X_OK):
            raise RuntimeError("helper is not executable")

    def launch(self, lifetime=30, allowed=True):
        parent, child = socket.socketpair()
        self.addCleanup(parent.close)
        parent.settimeout(8)
        # dup2 after Python startup, then exec: works even when fd 3 was occupied
        # in the parent. No credentials/policy in argv, env or disk.
        launcher = (
            "import os,sys; fd=int(sys.argv[1]); "
            "os.dup2(fd,3,inheritable=True); "
            "os.execv(sys.argv[2],[sys.argv[2]])"
        )
        try:
            process = subprocess.Popen(
                [sys.executable, "-c", launcher, str(child.fileno()), str(self.helper)],
                pass_fds=(child.fileno(),), stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
        finally:
            child.close()
        def reap():
            parent.close()
            if process.poll() is None:
                process.terminate()
            try:
                process.wait(timeout=8)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=8)
        self.addCleanup(reap)
        send_frame(parent, {
            "config": {"permission_version": 7, "users": {
                f"uid:{os.geteuid()}": {"locked_use": True, "applications": {
                    "com.shiju.runtime-fixture": "allow" if allowed else "deny"
                }}
            }},
            "target": {"kind": "native_app", "bundle_id": "com.shiju.runtime-fixture",
                       "process_id": os.getpid()},
            "lifetime_seconds": lifetime,
        })
        return process, parent

    def ready(self, lifetime=30):
        process, lease = self.launch(lifetime)
        ready = receive_frame(lease)
        # Avoid assertions that print a secret-bearing ready payload.
        self.assertTrue(set(ready) == {
            "socket_path", "session_id", "credential", "permission_version"})
        self.assertTrue(bool(ready["credential"]))
        self.assertEqual(ready["permission_version"], 7)
        return process, lease, ready

    def connect(self, ready):
        channel = socket.socket(socket.AF_UNIX)
        self.addCleanup(channel.close)
        channel.settimeout(8)
        channel.connect(ready["socket_path"])
        return channel

    def hello(self, channel, ready, credential=None):
        send_frame(channel, {"message": "hello", "session_id": ready["session_id"],
                             "credential": ready["credential"] if credential is None else credential})
        return receive_frame(channel)

    def test_missing_service(self):
        with tempfile.TemporaryDirectory(prefix="shiju-absent-") as root:
            with socket.socket(socket.AF_UNIX) as channel:
                with self.assertRaises(FileNotFoundError):
                    channel.connect(str(Path(root) / "absent.sock"))

    def test_authenticated_private_socket(self):
        _, _, ready = self.ready()
        path = Path(ready["socket_path"])
        self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
        self.assertEqual(stat.S_IMODE(path.parent.stat().st_mode), 0o700)
        self.assertEqual(path.stat().st_uid, os.geteuid())
        self.assertEqual(self.hello(self.connect(ready), ready), {"message": "ready"})

    def test_wrong_credential_and_one_use_ticket(self):
        _, _, ready = self.ready()
        denied = {"message": "rejected", "error": "permission_denied"}
        self.assertEqual(self.hello(self.connect(ready), ready, "invalid"), denied)
        self.assertEqual(self.hello(self.connect(ready), ready), {"message": "ready"})
        self.assertEqual(self.hello(self.connect(ready), ready), denied)

    def test_incompatible_protocol(self):
        _, _, ready = self.ready()
        channel = self.connect(ready)
        send_frame(channel, {"message": "hello_v999"})
        self.assertEqual(receive_frame(channel),
                         {"message": "rejected", "error": "invalid_request"})

    def test_repeated_launch_isolated_and_lease_revocation(self):
        first, lease, a = self.ready()
        second, _, b = self.ready()
        self.assertNotEqual(a["socket_path"], b["socket_path"])
        self.assertTrue(a["session_id"] != b["session_id"])
        lease.close()  # Trusted launcher's revocation/shutdown contract.
        self.assertEqual(first.wait(timeout=8), 0)
        self.assertFalse(Path(a["socket_path"]).exists())
        self.assertIsNone(second.poll())
        self.assertEqual(self.hello(self.connect(b), b), {"message": "ready"})

    def test_session_expiry(self):
        process, _, ready = self.ready(lifetime=2)
        self.assertEqual(process.wait(timeout=8), 0)
        self.assertFalse(Path(ready["socket_path"]).exists())

    def test_explicit_denial_prevents_service(self):
        process, lease = self.launch(allowed=False)
        with self.assertRaises((EOFError, ConnectionResetError)):
            receive_frame(lease)
        self.assertNotEqual(process.wait(timeout=8), 0)


if __name__ == "__main__":
    unittest.main()
