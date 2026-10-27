FROM vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf
WORKDIR /app
# One pinned amd64 GNU cross-binutils package; dependencies already satisfy dpkg.
# Offline install: no apt indexes, upgrades, resolver downloads or parent material.
COPY binutils-mipsel-linux-gnu_2.40-2cross2_amd64.deb /tmp/mipsel.deb
RUN printf '%s  %s\n' e803da3a36d96b7e16ae54a0e84c850a10e959d39f4a83efd02bcadc18db9e11 /tmp/mipsel.deb | sha256sum -c - \
 && dpkg --install /tmp/mipsel.deb \
 && dpkg-query -W binutils-mipsel-linux-gnu \
 && mipsel-linux-gnu-as --version \
 && mipsel-linux-gnu-ld --version \
 && python3 -c "from pathlib import Path; Path('/tmp/mipsel.deb').unlink()"
