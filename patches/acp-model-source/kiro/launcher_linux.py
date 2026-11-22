#!/usr/bin/env -S python3 -I
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
if hashlib.sha256(server.read_bytes()).hexdigest() != "8faffef164c344149ae95df432de93f1bd224c7f759e8f17b250e68f7e79c50c":
    raise SystemExit("Kiro Linux server checksum mismatch")
env = dict(os.environ)
if "VW_ACP_LAUNCH_REFERENCE" in env:
    required = (
        "KIRO_HOME", "KIRO_RUNTIME_DIR", "KIRO_KAS_ENDPOINT",
        "KIRO_KAS_CONTROL_PLANE_ENDPOINT", "KIRO_KAS_REGION", "KIRO_API_KEY",
        "KIRO_NO_AUTO_UPDATE", "VW_ACP_LAUNCH_REFERENCE",
    )
    if not all(env.get(key) for key in required):
        raise SystemExit("Incomplete managed Kiro launch")
    allowed = (*required, "SHIJU_KIRO_SKILL_ROOT", "LANG", "LC_ALL", "TMPDIR")
    env = {key: env[key] for key in allowed if key in env}
    env.update(HOME=env["KIRO_HOME"], PATH=os.defpath)
env.update(KIRO_KAS_SERVER_PATH=str(server), KIRO_KAS_NODE_PATH=str(root / "engine/node"))
native = str(root / "native/kiro-cli")
os.execve(native, [native, *sys.argv[1:]], env)
