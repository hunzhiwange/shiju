#!/bin/sh
# Runs only after the agent exits. No task artifacts or scorer files are changed.
set -eu
cd /opt/vw-verifier-deps
tar -xf python-local.tar
python3.13 -c 'import ssl, sqlite3, bz2, lzma; import sys; assert sys.version_info[:3] == (3, 13, 7); print(sys.version)'
# Official test.sh downloads the unmodified uv 0.9.5 archive through UV_DOWNLOAD_URL.
