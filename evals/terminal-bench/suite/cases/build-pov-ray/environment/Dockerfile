FROM vw-cython-diagnostics@sha256:e9ee4d2ebe7c9ff17917ef72979da36ee52876f5fc098ee1338db029f4cbe247
# Build context contains only the verified public dependency binary; no task,
# source tree, checker, reference image, oracle or solution is copied here.
COPY --chmod=755 povray /opt/legacy/povray
WORKDIR /app
