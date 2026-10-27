#!/bin/sh
# Verifier-only dependency cache; leaves /app and official scoring untouched.
set -eu
cd /opt/vw-verifier-deps
tar -xf python-local.tar
mkdir -p local/bin
tar -xf uv-x86_64-unknown-linux-gnu.tar.gz --strip-components=1 -C local/bin
local/bin/uv --version
local/bin/python3.13 -c 'import sys; assert sys.version_info[:3] == (3, 13, 7)'
