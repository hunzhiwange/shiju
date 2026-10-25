# Build only with the task25-verified Linux ELF and a held repository Cargo slot.
# runtime-slot is a named context pointing at the absolute CARGO_TARGET_DIR.
FROM alexgshaw/fix-git@sha256:389b9c8247610c2c5be080b1ac00429007c2c69bf57f7f26c79f0f75ba2d5c74
ARG CARGO_TARGET_DIR
ARG SOURCE_COMMIT
ARG SOURCE_SHA256
LABEL org.opencontainers.image.revision=$SOURCE_COMMIT \
      dev.vibewindow.source.sha256=$SOURCE_SHA256
RUN --mount=type=bind,from=runtime-slot,target=${CARGO_TARGET_DIR} \
    test -n "$CARGO_TARGET_DIR" && mkdir -p /opt/vw && \
    cp "$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/debug/shiju" /opt/vw/shiju
# Context must contain only the four task25-verified runtime sidecars.
COPY libsqlite3.so libsqlite3.so.0 ca-certificates.crt openssl.cnf /opt/vw-libs/
ENV HOME=/tmp/vw-home RUST_LOG=warn LD_LIBRARY_PATH=/opt/vw-libs \
    SSL_CERT_FILE=/opt/vw-libs/ca-certificates.crt \
    OPENSSL_CONF=/opt/vw-libs/openssl.cnf
WORKDIR /tmp
ENTRYPOINT ["/bin/sh", "-c", "mkdir -p /tmp/vw-home /tmp/vw-config; exec /opt/vw/shiju --config-dir /tmp/vw-config gateway --host 127.0.0.1 --port 42619"]
