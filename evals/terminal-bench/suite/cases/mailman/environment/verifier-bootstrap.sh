#!/bin/sh
# Verifier phase only: authentic cached installer/release and offline wheels.
set -eu
cd /opt/vw-verifier-deps
mkdir -p bin
# Serve only the exact original installer request from SHA-bound cached bytes.
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
# The original official installer runs in test.sh and obtains the real release
# through curl file://, preserving the installer's status and normal env file.
tar --no-same-owner -xf wheels.tar
python3 -c 'import sys,sqlite3; assert sys.version_info[:3] == (3,12,3)'
