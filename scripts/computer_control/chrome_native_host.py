#!/usr/bin/env python3
"""Bounded Native Messaging frame relay. No tickets, permissions or commands stored.

Install only into ~/.shiju/computer-control (including its isolated Chrome profile).
The launcher must bind chrome.sock in that private directory with mode 0600.
Chrome's origin argument and the Rust hello are both checked; neither grants policy.
"""
import json
import os
from pathlib import Path
import selectors
import socket
import stat
import struct
import sys

HOST_NAME = "com.shiju.computer_control"
EXTENSION_ID = "ogfgcnpdhnmkpcbaclbglefaclanipmg"
ORIGIN = f"chrome-extension://{EXTENSION_ID}/"
MAX_FRAME = 1024 * 1024


class Frames:
    def __init__(self, endian):
        self.endian = endian
        self.buffer = bytearray()

    def feed(self, data):
        self.buffer.extend(data)
        messages = []
        while len(self.buffer) >= 4:
            size = struct.unpack(self.endian + "I", self.buffer[:4])[0]
            if size == 0 or size > MAX_FRAME:
                raise ValueError("invalid frame length")
            if len(self.buffer) < size + 4:
                break
            messages.append(bytes(self.buffer[4:4 + size]))
            del self.buffer[:4 + size]
        return messages


def private(path, mode, kind):
    info = path.lstat()
    if info.st_uid != os.geteuid() or stat.S_IMODE(info.st_mode) != mode or not kind(info.st_mode):
        raise ValueError("unsafe relay path")


def relay(origin, directory):
    if origin != ORIGIN:
        raise ValueError("unapproved extension origin")
    private(directory, 0o700, stat.S_ISDIR)
    endpoint = directory / "chrome.sock"
    private(endpoint, 0o600, stat.S_ISSOCK)
    with socket.socket(socket.AF_UNIX) as connection:
        connection.connect(str(endpoint))
        # Check the connected server as well as the socket file (replacement races).
        if sys.platform == "darwin":
            import ctypes
            uid, gid = ctypes.c_uint(), ctypes.c_uint()
            if ctypes.CDLL(None).getpeereid(connection.fileno(), ctypes.byref(uid), ctypes.byref(gid)) != 0:
                raise ValueError("peer lookup failed")
            peer = uid.value
        else:
            peer = struct.unpack("3i", connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))[1]
        if peer != os.geteuid():
            raise ValueError("unapproved local peer")
        with selectors.DefaultSelector() as selector:
            selector.register(sys.stdin.fileno(), selectors.EVENT_READ, (Frames("="), ">"))
            selector.register(connection, selectors.EVENT_READ, (Frames(">"), "="))
            while True:
                for key, _ in selector.select():
                    data = os.read(key.fd, 65536)
                    if not data:
                        # EOF, including a partial frame, tears down the whole session.
                        return
                    parser, output_endian = key.data
                    for payload in parser.feed(data):
                        frame = struct.pack(output_endian + "I", len(payload)) + payload
                        if key.fd == sys.stdin.fileno():
                            connection.sendall(frame)
                        else:
                            sys.stdout.buffer.write(frame)
                            sys.stdout.buffer.flush()


def install(directory):
    """Own only the relay and its manifest; never touch the normal Chrome profile."""
    base = Path.home() / ".shiju" / "computer-control"
    # Resolve parents before creating; reject symlink redirection out of ShiJu.
    if directory.absolute() != base.absolute() or directory.resolve() != Path.home().resolve() / ".shiju" / "computer-control":
        raise ValueError("installation must use the ShiJu user directory")
    directory.mkdir(parents=True, exist_ok=True, mode=0o700)
    private(directory, 0o700, stat.S_ISDIR)
    script = directory / "chrome_native_host.py"
    manifest_dir = directory / "chrome-profile" / "NativeMessagingHosts"
    manifest_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
    if manifest_dir.resolve() != directory.resolve() / "chrome-profile" / "NativeMessagingHosts":
        raise ValueError("symlink registration directory")
    manifest = manifest_dir / f"{HOST_NAME}.json"
    # Exclusive creation prevents overwriting an unrelated registration or symlink.
    content = Path(__file__).read_text()
    content = "#!" + sys.executable + "\n" + content.split("\n", 1)[1]
    with script.open("x") as output:
        output.write(content)
    script.chmod(0o700)
    try:
        with manifest.open("x") as output:
            json.dump({"name": HOST_NAME, "description": "ShiJu Chrome frame relay", "path": str(script),
                       "type": "stdio", "allowed_origins": [ORIGIN]}, output)
        manifest.chmod(0o600)
    except BaseException:
        script.unlink()
        raise
    return manifest


def uninstall(directory):
    base = Path.home() / ".shiju" / "computer-control"
    if directory.absolute() != base.absolute() or directory.resolve() != Path.home().resolve() / ".shiju" / "computer-control":
        raise ValueError("removal must use the ShiJu user directory")
    manifest = directory / "chrome-profile" / "NativeMessagingHosts" / f"{HOST_NAME}.json"
    if manifest.parent.resolve() != directory.resolve() / "chrome-profile" / "NativeMessagingHosts":
        raise ValueError("symlink registration directory")
    if manifest.exists():
        private(manifest, 0o600, stat.S_ISREG)
        registration = json.loads(manifest.read_text())
        if registration.get("path") != str(directory / "chrome_native_host.py") or registration.get("allowed_origins") != [ORIGIN]:
            raise ValueError("foreign registration")
        manifest.unlink()
    script = directory / "chrome_native_host.py"
    if script.exists():
        private(script, 0o700, stat.S_ISREG)
        script.unlink()


if __name__ == "__main__":
    try:
        if len(sys.argv) == 2 and sys.argv[1] in ("--install", "--uninstall"):
            root = Path.home() / ".shiju" / "computer-control"
            (install if sys.argv[1] == "--install" else uninstall)(root)
        elif len(sys.argv) >= 2 and len(sys.argv) <= 3:
            relay(sys.argv[1], Path(__file__).resolve().parent)
        else:
            raise ValueError("invalid relay invocation")
    except (OSError, ValueError):
        # Never expose payloads, paths or credentials to Chrome or stderr.
        sys.exit(1)
