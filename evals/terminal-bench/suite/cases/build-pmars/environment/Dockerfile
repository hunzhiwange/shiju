# Diagnostic tools only; no parent warriors, oracle, tests or solution.
FROM python@sha256:fcf02c9e248b995ae2ca9aac6fa24b489f34b589dbfdc44698ad245d2ad41d1e
RUN apt-get update && apt-get install -y --no-install-recommends gcc make libc6-dev libncurses-dev && rm -rf /var/lib/apt/lists/*
ADD https://deb.debian.org/debian/pool/main/p/pmars/pmars_0.9.2.orig.tar.gz /tmp/pmars.tar.gz
RUN echo 'df7f0a1d0026c4af4f57e941d1dd3c487be1eb8097a658cec12e1c1f8e8d3ead  /tmp/pmars.tar.gz' | sha256sum -c - && tar -xzf /tmp/pmars.tar.gz -C /opt && rm /tmp/pmars.tar.gz && cd /opt/pmars-0.9.2/src && sed -i 's/-DXWINGRAPHX//g; s/^LIB *=.*/LIB = -lncurses/' Makefile && make && install -m 0755 pmars /usr/local/bin/pmars && make clean && rm pmars
WORKDIR /app
