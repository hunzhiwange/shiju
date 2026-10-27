FROM --platform=linux/amd64 vw-win311-base@sha256:7a6db18e11071ff5a05ea8259da78073bd5730b85d39399511385e2a684749d7
RUN apt-get update && apt-get install -y --no-install-recommends vim && rm -rf /var/lib/apt/lists/*
WORKDIR /app
