#!/bin/sh
# Verifier-only dependency staging after the agent has exited.
set -eu
cd /opt/vw-verifier-deps
tar --no-same-owner -xf installer-tools.tar
tar --no-same-owner -xf wheels.tar
env TAR_OPTIONS=--no-same-owner PATH="/opt/vw-verifier-deps/installer-tools/bin:/usr/bin:/bin" LD_LIBRARY_PATH="/opt/vw-verifier-deps/installer-tools/lib" sh /opt/vw-verifier-deps/uv-installer.sh
printf '%s\n' 'd3dc3ca8e29337dd602a9a4df9e6edb15ebc52aed91461debeedb96939dc4ce0  /root/.local/bin/uv' '421b1bb58429180a04ade98da066b5a5b6763e8ed74edef97359701c4cb42db8  /root/.local/bin/uvx' | sha256sum -c -
# Bound redundant network installs while retaining real command failures.
# Official tests and reward computation remain byte-identical.
printf '%s\n' 'Acquire::Retries "0";' 'Acquire::http::Timeout "15";' 'Acquire::https::Timeout "15";' > /etc/apt/apt.conf.d/99vw-verifier-network
mkdir -p /opt/vw-verifier-deps/bin
cat > /opt/vw-verifier-deps/bin/apt-get <<'EOF'
#!/bin/sh
exec /usr/bin/timeout --kill-after=5s 30s /usr/bin/apt-get "$@"
EOF
cat > /opt/vw-verifier-deps/bin/curl <<'EOF'
#!/bin/sh
exec /opt/vw-verifier-deps/installer-tools/bin/curl --connect-timeout 15 --max-time 60 "$@"
EOF
chmod 755 /opt/vw-verifier-deps/bin/apt-get /opt/vw-verifier-deps/bin/curl
curl --version
