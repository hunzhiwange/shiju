#!/bin/sh
# Dependency provisioning runs only after the agent; no task/scorer files change.
set -eu
cd /opt/vw-verifier-deps
tar -xf uv-x86_64-unknown-linux-gnu.tar.gz
mkdir -p "$HOME/.local/bin"
ln -sf /opt/vw-verifier-deps/uv-x86_64-unknown-linux-gnu/uv "$HOME/.local/bin/uv"
ln -sf /opt/vw-verifier-deps/uv-x86_64-unknown-linux-gnu/uvx "$HOME/.local/bin/uvx"
printf '%s\n' 'export PATH="$HOME/.local/bin:$PATH"' > "$HOME/.local/bin/env"
"$HOME/.local/bin/uv" --version
python3 -c 'import sys; assert sys.version_info[:2] == (3, 13)'
