#!/bin/sh
# Runs only after the agent exits; installs authentic cached verifier tools.
set -eu
cd /opt/vw-verifier-deps
tar --no-same-owner -xf installer-tools.tar
tar --no-same-owner -xf wheels.tar
env TAR_OPTIONS=--no-same-owner PATH="/opt/vw-verifier-deps/installer-tools/bin:/usr/bin:/bin" LD_LIBRARY_PATH="/opt/vw-verifier-deps/installer-tools/lib" sh /opt/vw-verifier-deps/uv-installer.sh
printf '%s\n' 'd3dc3ca8e29337dd602a9a4df9e6edb15ebc52aed91461debeedb96939dc4ce0  /root/.local/bin/uv' '421b1bb58429180a04ade98da066b5a5b6763e8ed74edef97359701c4cb42db8  /root/.local/bin/uvx' | sha256sum -c -
