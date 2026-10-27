#!/bin/sh
# Only verifier dependencies enter this directory, after the agent exits.
set -eu
cd /opt/vw-verifier-deps
tar --no-same-owner -xf python-local.tar
tar --no-same-owner -xf native-libs.tar
cp native-libs/*.so* local/lib/
tar --no-same-owner -xf wheels.tar
python3.13 -c 'import ssl, sqlite3, bz2, lzma; import sys; assert sys.version_info[:3] == (3, 13, 7); print(sys.version)'
# The unmodified official installer consumes the original uv release archive.
# UV_OFFLINE and UV_FIND_LINKS make the official uvx resolve local wheels only.
tar --no-same-owner -xf installer-tools.tar
# Execute the byte-identical official installer with a real cached curl.
# The curl closure is scoped to this subprocess; R/Python keep their own libraries.
env TAR_OPTIONS=--no-same-owner PATH="/opt/vw-verifier-deps/installer-tools/bin:/usr/bin:/bin" LD_LIBRARY_PATH="/opt/vw-verifier-deps/installer-tools/lib" sh /opt/vw-verifier-deps/uv-installer.sh
printf '%s\n' 'd3dc3ca8e29337dd602a9a4df9e6edb15ebc52aed91461debeedb96939dc4ce0  /root/.local/bin/uv' '421b1bb58429180a04ade98da066b5a5b6763e8ed74edef97359701c4cb42db8  /root/.local/bin/uvx' | sha256sum -c -
