FROM vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf
COPY packages/ /tmp/coq-debs/
RUN DEBIAN_FRONTEND=noninteractive dpkg -i /tmp/coq-debs/libpython3.11-minimal_*.deb /tmp/coq-debs/python3.11-minimal_*.deb && dpkg -i /tmp/coq-debs/python3-minimal_*.deb && dpkg -i /tmp/coq-debs/*.deb && rm -rf /tmp/coq-debs && coqc --version && coqchk --version && python3 -V
WORKDIR /app
