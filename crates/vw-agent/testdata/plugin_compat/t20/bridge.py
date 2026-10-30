"""MIT: test-owned stdio to loopback bridge for the real Rust rmcp test service."""
import socket
import os
import sys
import threading

connection = socket.create_connection(("127.0.0.1", int(sys.argv[1])))

def forward_input():
    try:
        while data := os.read(sys.stdin.fileno(), 65536):
            connection.sendall(data)
    except OSError:
        pass
    finally:
        try:
            connection.shutdown(socket.SHUT_WR)
        except OSError:
            pass

threading.Thread(target=forward_input, daemon=True).start()
with connection.makefile("rb") as stream:
    for line in stream:
        sys.stdout.buffer.write(line)
        sys.stdout.buffer.flush()
