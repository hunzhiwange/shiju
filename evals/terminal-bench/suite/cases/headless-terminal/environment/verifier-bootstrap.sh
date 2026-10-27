#!/bin/sh
# Runs only after the agent exits; keep the original Python and official tests.
set -eu
cd /opt/vw-verifier-deps
tar --no-same-owner -xf wheels.tar
python -c 'import sys; assert sys.version_info[:3] == (3, 13, 7)'
# Bound dependency networking while retaining real apt commands and exit status.
printf '%s\n' 'Acquire::Retries "0";' 'Acquire::http::Timeout "15";' 'Acquire::https::Timeout "15";' > /etc/apt/apt.conf.d/99vw-verifier-network
mkdir -p bin
cat > bin/apt-get <<'EOF'
#!/bin/sh
exec /usr/bin/timeout --kill-after=5s 90s /usr/bin/apt-get "$@"
EOF
chmod 755 bin/apt-get
printf '%s\n' '[global]' 'no-index = true' 'find-links = /opt/vw-verifier-deps/wheels' 'disable-pip-version-check = true' > /etc/pip.conf
