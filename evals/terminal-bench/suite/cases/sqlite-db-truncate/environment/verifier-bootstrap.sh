#!/bin/sh
# Verifier phase only: authentic uv 0.9.5 and pinned pytest wheel cache.
set -eu
cd /opt/vw-verifier-deps
mkdir -p bin
tar --no-same-owner -xf wheels.tar
cat > bin/curl <<'EOF'
#!/bin/sh
if [ "$#" = 2 ] && [ "$1" = '-LsSf' ] && [ "$2" = 'https://astral.sh/uv/0.9.5/install.sh' ]; then
    exec cat /opt/vw-verifier-deps/uv-installer.sh
fi
if [ "$#" = 4 ] && [ "$1" = '-sSfL' ] && [ "$2" = 'file:///opt/vw-verifier-deps/uv-x86_64-unknown-linux-gnu.tar.gz' ] && [ "$3" = '-o' ]; then
    exec cp /opt/vw-verifier-deps/uv-x86_64-unknown-linux-gnu.tar.gz "$4"
fi
exec /usr/bin/curl "$@"
EOF
cat > bin/apt-get <<'EOF'
#!/bin/sh
exec /usr/bin/timeout --kill-after=5s 30s /usr/bin/apt-get "$@"
EOF
chmod 755 bin/curl bin/apt-get
# Keep the official signed Debian repositories; bound redundant online setup.
sed -i 's|http://deb.debian.org|https://deb.debian.org|g' /etc/apt/sources.list.d/debian.sources
printf '%s\n' 'Acquire::Retries "0";' 'Acquire::http::Timeout "15";' 'Acquire::https::Timeout "15";' > /etc/apt/apt.conf.d/99vw-verifier-network
TAR_OPTIONS=--no-same-owner sh uv-installer.sh
printf '%s\n' 'd3dc3ca8e29337dd602a9a4df9e6edb15ebc52aed91461debeedb96939dc4ce0  /root/.local/bin/uv' '421b1bb58429180a04ade98da066b5a5b6763e8ed74edef97359701c4cb42db8  /root/.local/bin/uvx' | sha256sum -c -
/root/.local/bin/uvx -p 3.13 -w pytest==8.4.1 -w pytest-json-ctrf==0.3.5 python -c 'import sys,pytest,ctrf; assert sys.version_info[:2]==(3,13) and pytest.__version__=="8.4.1"; print("verifier dependency closure verified",sys.version)'
