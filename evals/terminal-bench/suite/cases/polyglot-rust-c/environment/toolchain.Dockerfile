FROM vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf
COPY rustc-1.75.0-x86_64-unknown-linux-gnu.tar.xz rust-std-1.75.0-x86_64-unknown-linux-gnu.tar.xz /tmp/rust-distribution/
COPY install.py /tmp/rust-distribution/install.py
RUN python3 /tmp/rust-distribution/install.py && rm -rf /tmp/rust-distribution
ENV PATH=/opt/rust75/bin:/usr/local/bin:/usr/local/sbin:/usr/sbin:/usr/bin:/sbin:/bin
WORKDIR /app
