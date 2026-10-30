#!/bin/sh
# Verifier-only authentic Python/uv and SHA-bound offline wheel transport.
set -eu
cd /opt/vw-verifier-deps
tar --no-same-owner -xf installer-tools.tar
tar --no-same-owner -xf wheels.tar
tar --no-same-owner -xf python-local.tar
cp libsqlite3.so.0 local/lib/libsqlite3.so.0
mv ./*.whl wheels/
env TAR_OPTIONS=--no-same-owner PATH="/opt/vw-verifier-deps/installer-tools/bin:/usr/bin:/bin" LD_LIBRARY_PATH="/opt/vw-verifier-deps/installer-tools/lib" sh ./uv-installer.sh
printf '%s\n' 'd3dc3ca8e29337dd602a9a4df9e6edb15ebc52aed91461debeedb96939dc4ce0  /root/.local/bin/uv' '421b1bb58429180a04ade98da066b5a5b6763e8ed74edef97359701c4cb42db8  /root/.local/bin/uvx' | sha256sum -c -
printf '%s\n' 'Acquire::Retries "0";' 'Acquire::http::Timeout "15";' 'Acquire::https::Timeout "15";' > /etc/apt/apt.conf.d/99vw-verifier-network
mkdir -p bin
cat > bin/apt-get <<'SCRIPT'
#!/bin/sh
exec /usr/bin/timeout --kill-after=5s 90s /usr/bin/apt-get "$@"
SCRIPT
chmod 755 bin/apt-get
curl --version
python3.13 -c 'import ssl, sqlite3, bz2, lzma; import sys; assert sys.version_info[:3] == (3, 13, 7)'
