# Public diagnostic tools only. No parent tests/solution or diagnostic gold.
FROM --platform=linux/amd64 python@sha256:3f1ce1c415054488586812b63270daf89d97b4f23416e1288003816790eb08de
RUN apt-get update && apt-get install -y --no-install-recommends gcc make libc6-dev binutils libgmp10 && rm -rf /var/lib/apt/lists/*
# Build context: this task's ignored evidence root. Inputs must be exported
# from a normally completed, freshly built, passing original parent oracle.
# compcert.ini is regenerated from its pinned public recipe and parent summary.
COPY public-toolchain-2026-10-28/ccomp /usr/local/bin/ccomp
COPY public-toolchain-2026-10-28/compcert/ /usr/local/lib/compcert/
COPY public-toolchain-2026-10-28/compcert.ini /usr/local/share/compcert.ini
COPY public-toolchain-2026-10-28/LICENSE /usr/share/doc/compcert/LICENSE
WORKDIR /app
CMD ["python3"]
