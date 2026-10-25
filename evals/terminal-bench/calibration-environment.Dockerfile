# Task28 environment repair only; never copy tasks, solutions or verifier inputs.
ARG BASE_IMAGE=alexgshaw/git-multibranch@sha256:5bc449921c138295f6ddd8dca2977319fee2ce523811dbe56cec4ca663385fb5
FROM ${BASE_IMAGE}
RUN printf 'Acquire::Retries "5";\n' > /etc/apt/apt.conf.d/80-calibration-retries \
    && apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends tmux=3.4-1ubuntu0.2 \
    && dpkg-query -W tmux libevent-core-2.1-7t64 libutempter0 \
    && rm -rf /var/lib/apt/lists/*
