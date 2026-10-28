#!/bin/sh
# Verifier phase only. Keep the original test.sh, resolver and scorer unchanged.
set -eu
cd /opt/vw-verifier-deps
mkdir -p bin python
tar --no-same-owner -xf wheels.tar
tar --no-same-owner -xzf python313.tar.gz -C python
cat > bin/curl <<'CURL'
#!/bin/sh
if [ "$#" = 2 ] && [ "$1" = '-LsSf' ] && [ "$2" = 'https://astral.sh/uv/0.9.5/install.sh' ]; then
    exec cat /opt/vw-verifier-deps/uv-installer.sh
fi
exec /usr/bin/curl "$@"
CURL
cat > bin/apt-get <<'APT'
#!/bin/sh
exec /usr/bin/timeout --kill-after=5s 90s /usr/bin/apt-get "$@"
APT
chmod 755 bin/curl bin/apt-get
printf '%s\n' 'Acquire::Retries "0";' 'Acquire::http::Timeout "15";' 'Acquire::https::Timeout "15";' > /etc/apt/apt.conf.d/99vw-verifier-network
# Probe the authentic relocated CPython and the real system SQLite closure.
python/bin/python3.13 -c 'import sys,sqlite3,ssl,ctypes; assert sys.version_info[:3] == (3,13,7); print(sys.version, sqlite3.sqlite_version, ssl.OPENSSL_VERSION)'
