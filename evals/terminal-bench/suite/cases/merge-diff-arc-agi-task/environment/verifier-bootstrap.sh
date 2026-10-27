#!/bin/sh
# Dependency-only bootstrap after the agent phase; official tests are unchanged.
set -eu
cd /opt/vw-verifier-deps
tar --no-same-owner -xf python-local.tar
mkdir -p bin
cat > bin/curl <<'EOF'
#!/bin/sh
if [ "$#" = 2 ] && [ "$1" = '-LsSf' ] && [ "$2" = 'https://astral.sh/uv/0.9.5/install.sh' ]; then
    exec cat /opt/vw-verifier-deps/uv-installer.sh
fi
exec /usr/bin/curl "$@"
EOF
cat > bin/apt-get <<'EOF'
#!/bin/sh
exec /usr/bin/timeout --kill-after=5s 90s /usr/bin/apt-get "$@"
EOF
chmod 755 bin/curl bin/apt-get
printf '%s\n' 'Acquire::Retries "0";' 'Acquire::http::Timeout "15";' 'Acquire::https::Timeout "15";' > /etc/apt/apt.conf.d/99vw-verifier-network
apt-get update
apt-get install -y libsqlite3-0 libbz2-1.0 liblzma5
python3.13 -c 'import ssl,sqlite3,bz2,lzma,sys; assert sys.version_info[:3] == (3,13,7)'
