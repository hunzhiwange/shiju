#!/bin/sh
# Runs only after the agent exits; installs authentic cached verifier tools.
set -eu
cd /opt/vw-verifier-deps
tar --no-same-owner -xf installer-tools.tar
tar --no-same-owner -xf wheels.tar
env TAR_OPTIONS=--no-same-owner PATH="/opt/vw-verifier-deps/installer-tools/bin:/usr/bin:/bin" LD_LIBRARY_PATH="/opt/vw-verifier-deps/installer-tools/lib" sh /opt/vw-verifier-deps/uv-installer.sh
printf '%s\n' 'd3dc3ca8e29337dd602a9a4df9e6edb15ebc52aed91461debeedb96939dc4ce0  /root/.local/bin/uv' '421b1bb58429180a04ade98da066b5a5b6763e8ed74edef97359701c4cb42db8  /root/.local/bin/uvx' | sha256sum -c -
python3.13 -c 'import ssl, sqlite3, bz2, lzma; import sys; assert sys.version_info[:3] == (3, 13, 7)'
# Official dependency bootstrap uses HTTP indexes that can exceed the verifier
# limit here. Preserve repositories and signatures; use their HTTPS endpoints.
sed -i 's|http://deb.debian.org|https://deb.debian.org|g' /etc/apt/sources.list.d/debian.sources
printf '%s\n' 'Acquire::Retries "0";' 'Acquire::http::Timeout "15";' 'Acquire::https::Timeout "15";' > /etc/apt/apt.conf.d/99vw-verifier-network
# Authentic cached uv/Python/pytest/curl are already installed. Bound redundant
# online apt bootstrap calls within the original verifier budget. This wrapper
# runs the real apt-get, preserves nonzero/timeout failure, and never grades.
mkdir -p /opt/vw-verifier-deps/bin
cat > /opt/vw-verifier-deps/bin/apt-get <<'EOF'
#!/bin/sh
exec /usr/bin/timeout --kill-after=5s 30s /usr/bin/apt-get "$@"
EOF
chmod 755 /opt/vw-verifier-deps/bin/apt-get
command -v timeout
curl --version
