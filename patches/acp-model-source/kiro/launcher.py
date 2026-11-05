#!/usr/bin/env python3
"""Launch only the independently reviewed native Kiro and extracted V3 engine."""
import hashlib
import os
from pathlib import Path
import sys

root = Path(__file__).resolve().parent
# These are installation-owned paths, never caller-controlled engine overrides.
for key in ('KIRO_KAS_SERVER_PATH', 'KIRO_KAS_NODE_PATH', 'KAS_BUNDLE_PATH'):
    if key in os.environ:
        raise SystemExit('Conflicting Kiro engine override')
server = root / 'runtime/3.0.0-96d7248d901e42e1/kas/node_modules/@kiro/agent/dist/server/acp-server.js'
if hashlib.sha256(server.read_bytes()).hexdigest() != '26907b1f2895cf28a850d7720b340cb206965d8ffbc17cb62dd78276f2dbd02d':
    raise SystemExit('Kiro reviewed engine checksum mismatch')
env = dict(os.environ, KIRO_KAS_SERVER_PATH=str(server),
           KIRO_KAS_NODE_PATH=str(root / 'runtime/3.0.0-96d7248d901e42e1/node'))
os.execve(str(root / 'native/kiro-cli'), [str(root / 'native/kiro-cli'), *sys.argv[1:]], env)
