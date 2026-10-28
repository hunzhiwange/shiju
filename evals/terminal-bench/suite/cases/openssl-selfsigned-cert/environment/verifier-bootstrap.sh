#!/bin/sh
# Verifier phase only: authentic cached installer, Linux curl and pytest wheels.
set -eu
cd /opt/vw-verifier-deps
tar --no-same-owner -xf installer-tools.tar
tar --no-same-owner -xf wheels.tar
mkdir -p bin
cat > bin/curl <<'EOF'
#!/bin/sh
if [ "$#" = 2 ] && [ "$1" = '-LsSf' ] && [ "$2" = 'https://astral.sh/uv/0.9.5/install.sh' ]; then
    exec cat /opt/vw-verifier-deps/uv-installer.sh
fi
exec /opt/vw-verifier-deps/installer-tools/bin/curl "$@"
EOF
cat > bin/apt-get <<'EOF'
#!/bin/sh
exec /usr/bin/timeout --kill-after=5s 30s /usr/bin/apt-get "$@"
EOF
cat > bin/tar <<'EOF'
#!/bin/sh
export TAR_OPTIONS=--no-same-owner
exec /usr/bin/tar "$@"
EOF
chmod 755 bin/curl bin/apt-get bin/tar
printf '%s\n' 'Acquire::Retries "0";' 'Acquire::http::Timeout "15";' 'Acquire::https::Timeout "15";' > /etc/apt/apt.conf.d/99vw-verifier-network
python3 -c 'import sys, ssl, sqlite3, bz2, lzma; assert sys.version_info[:3] == (3, 13, 7); print(ssl.OPENSSL_VERSION, sqlite3.sqlite_version)'
curl --version
# Exercise the real pinned installer and release before the unmodified test.sh.
sh uv-installer.sh
/root/.local/bin/uv --version
/root/.local/bin/uvx -p 3.13 -w pytest==8.4.1 -w pytest-json-ctrf==0.3.5 python -c 'import sqlite3, pytest, ctrf; assert pytest.__version__ == "8.4.1"; print(sqlite3.sqlite_version)'
