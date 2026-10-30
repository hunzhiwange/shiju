FROM vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf
COPY packages/ /tmp/tex-debs/
RUN printf '#!/bin/sh\nexit 101\n' > /usr/sbin/policy-rc.d && chmod 755 /usr/sbin/policy-rc.d && DEBIAN_FRONTEND=noninteractive dpkg -i /tmp/tex-debs/*.deb && python3 -c "import shutil; shutil.rmtree('/tmp/tex-debs')" && pdflatex --version && python3 -V
WORKDIR /app
