FROM vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf
COPY packages/ /tmp/nginx-debs/
RUN printf '#!/bin/sh\nexit 101\n' > /usr/sbin/policy-rc.d && chmod 755 /usr/sbin/policy-rc.d && DEBIAN_FRONTEND=noninteractive dpkg -i /tmp/nginx-debs/*.deb && python3 -c "import pathlib,shutil; [shutil.rmtree(p) for p in [pathlib.Path('/tmp/nginx-debs')]]" && nginx -V && ldd /usr/sbin/nginx && python3 -V
WORKDIR /app
