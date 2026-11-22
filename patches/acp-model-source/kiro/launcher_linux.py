#!/usr/bin/env python3
"""Start the pinned Linux Kiro CLI with its private V3 runtime."""
import hashlib
import os
from pathlib import Path
import sys


root = Path(__file__).resolve().parent
for key in ("KIRO_KAS_SERVER_PATH", "KIRO_KAS_NODE_PATH", "KAS_BUNDLE_PATH"):
    if key in os.environ:
        raise SystemExit("Conflicting Kiro engine override")
server = root / "engine/kas/node_modules/@kiro/agent/dist/server/acp-server.js"
if hashlib.sha256(server.read_bytes()).hexdigest() != "8619822571719aab4031891bea39d0286ec7fef441ab71445a9042b963c95329":
    raise SystemExit("Kiro Linux server checksum mismatch")
env = dict(os.environ, KIRO_KAS_SERVER_PATH=str(server),
           KIRO_KAS_NODE_PATH=str(root / "engine/node"))
native = str(root / "native/kiro-cli")
os.execve(native, [native, *sys.argv[1:]], env)
