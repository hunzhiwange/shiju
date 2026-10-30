#!/bin/sh
# Verifier-only cached upstream tools; the official test script stays unchanged.
set -eu
cd /opt/vw-verifier-deps
tar --no-same-owner -xf installer-tools.tar
tar --no-same-owner -xf wheels.tar
tar --no-same-owner -xf python-local.tar
tar --no-same-owner -xf python-libs.tar
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
cat > bin/apt <<'EOF'
#!/bin/sh
exec /usr/bin/timeout --kill-after=5s 30s /usr/bin/apt "$@"
EOF
cat > bin/tar <<'EOF'
#!/bin/sh
export TAR_OPTIONS=--no-same-owner
exec /usr/bin/tar "$@"
EOF
chmod 755 bin/curl bin/apt-get bin/apt bin/tar
printf '%s\n' 'Acquire::Retries "0";' 'Acquire::http::Timeout "15";' 'Acquire::https::Timeout "15";' > /etc/apt/apt.conf.d/99vw-verifier-network
python3.13 -c 'import sys, ssl, sqlite3, bz2, lzma; assert sys.version_info[:3] == (3, 13, 7); print(ssl.OPENSSL_VERSION, sqlite3.sqlite_version)'
curl --version
sh uv-installer.sh
printf '%s\n' 'd3dc3ca8e29337dd602a9a4df9e6edb15ebc52aed91461debeedb96939dc4ce0  /root/.local/bin/uv' '421b1bb58429180a04ade98da066b5a5b6763e8ed74edef97359701c4cb42db8  /root/.local/bin/uvx' | sha256sum -c -
/root/.local/bin/uvx -p 3.13 -w pytest==8.4.1 -w pytest-json-ctrf==0.3.5 python -c 'import ssl, sqlite3, pytest, ctrf; assert pytest.__version__ == "8.4.1"; print(sqlite3.sqlite_version)'
/usr/bin/pdflatex --version
