# Tool-only image. No upstream task, checker or solution is included.
FROM python@sha256:fcf02c9e248b995ae2ca9aac6fa24b489f34b589dbfdc44698ad245d2ad41d1e
RUN apt-get update && apt-get install -y --no-install-recommends g++ valgrind && rm -rf /var/lib/apt/lists/*
WORKDIR /app
