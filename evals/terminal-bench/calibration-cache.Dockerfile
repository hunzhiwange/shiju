# Exact upstream dependency bytes; no task, solution or verifier files.
ARG BASE_IMAGE=alexgshaw/fix-git@sha256:389b9c8247610c2c5be080b1ac00429007c2c69bf57f7f26c79f0f75ba2d5c74
FROM ${BASE_IMAGE}
COPY uv-x86_64-unknown-linux-gnu.tar.gz /opt/calibration-downloads/uv-x86_64-unknown-linux-gnu.tar.gz
RUN echo '2cf10babba653310606f8b49876cfb679928669e7ddaa1fb41fb00ce73e64f66  /opt/calibration-downloads/uv-x86_64-unknown-linux-gnu.tar.gz' | sha256sum -c -
ENV UV_DOWNLOAD_URL=file:///opt/calibration-downloads
