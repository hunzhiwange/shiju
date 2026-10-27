#!/bin/sh
# Runs only after the agent exits; installs authentic cached verifier tools.
set -eu
cd /opt/vw-verifier-deps
tar --no-same-owner -xf installer-tools.tar
tar --no-same-owner -xf wheels.tar
env TAR_OPTIONS=--no-same-owner PATH="/opt/vw-verifier-deps/installer-tools/bin:/usr/bin:/bin" LD_LIBRARY_PATH="/opt/vw-verifier-deps/installer-tools/lib" sh /opt/vw-verifier-deps/uv-installer.sh
printf '%s\n' 'd3dc3ca8e29337dd602a9a4df9e6edb15ebc52aed91461debeedb96939dc4ce0  /root/.local/bin/uv' '421b1bb58429180a04ade98da066b5a5b6763e8ed74edef97359701c4cb42db8  /root/.local/bin/uvx' | sha256sum -c -
# Retain the original image's Python and headers for the official C extension.
python3 -c 'import sys,sysconfig; from pathlib import Path; assert sys.version_info[:3] == (3,13,7); assert Path(sysconfig.get_path("include"),"Python.h").is_file()'
sed -i 's|http://deb.debian.org|https://deb.debian.org|g' /etc/apt/sources.list.d/debian.sources
printf '%s\n' 'Acquire::Retries "0";' 'Acquire::http::Timeout "20";' 'Acquire::https::Timeout "20";' > /etc/apt/apt.conf.d/99vw-verifier-network
mkdir -p /opt/vw-verifier-deps/bin
cat > /opt/vw-verifier-deps/bin/apt-get <<'EOF'
#!/bin/sh
exec /usr/bin/timeout --kill-after=5s 120s /usr/bin/apt-get "$@"
EOF
chmod 755 /opt/vw-verifier-deps/bin/apt-get
# Authentic packages from APT-verified signed Debian indexes; no network fallback.
tar --no-same-owner -xf debs.tar
tar --no-same-owner -C /var/lib/apt/lists -xf apt-lists.tar
(cd debs && sha256sum -c SHA256SUMS)
cp debs/*.deb /var/cache/apt/archives/
/usr/bin/apt-get install -y --no-download --no-install-recommends gcc libc6-dev curl
gcc --version
