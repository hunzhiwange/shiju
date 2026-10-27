FROM python@sha256:fcf02c9e248b995ae2ca9aac6fa24b489f34b589dbfdc44698ad245d2ad41d1e
RUN apt-get update && apt-get install -y --no-install-recommends gcc libc6-dev \
    && rm -rf /var/lib/apt/lists/*
RUN python -m pip install --no-cache-dir numpy==2.3.0 Cython==3.1.3 setuptools==80.9.0
WORKDIR /app
