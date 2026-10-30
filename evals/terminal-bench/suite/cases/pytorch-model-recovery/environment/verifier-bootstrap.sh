#!/bin/sh
# Dependency setup only, injected by the standard adapter after agent execution.
set -eu
cd /opt/vw-verifier-deps
tar --no-same-owner -xzf uv-x86_64-unknown-linux-gnu.tar.gz
tar --no-same-owner -xf wheels.tar
mkdir -p /root/.local/bin
cp uv-x86_64-unknown-linux-gnu/uv uv-x86_64-unknown-linux-gnu/uvx /root/.local/bin/
printf '%s\n' 'export PATH="/root/.local/bin:$PATH"' > /root/.local/bin/env
/root/.local/bin/uv --version
# Keep the official APT bootstrap, using reachable HTTPS repositories and
# bounded real apt-get calls. Nonzero/timeout results remain nonzero.
sed -i 's|http://deb.debian.org|https://deb.debian.org|g; s|http://security.debian.org|https://security.debian.org|g' /etc/apt/sources.list.d/debian.sources
printf '%s\n' 'Acquire::Retries "0";' 'Acquire::http::Timeout "15";' 'Acquire::https::Timeout "15";' > /etc/apt/apt.conf.d/99vw-verifier-network
mkdir -p /opt/vw-verifier-deps/bin
cat > /opt/vw-verifier-deps/bin/apt-get <<'EOF'
#!/bin/sh
exec /usr/bin/timeout --kill-after=5s 60s /usr/bin/apt-get "$@"
EOF
chmod 755 /opt/vw-verifier-deps/bin/apt-get
command -v timeout
/root/.local/bin/uvx -p 3.13 -w pytest==8.4.1 -w torch==2.7.1 -w pytest-json-ctrf==0.3.5 pytest --version
