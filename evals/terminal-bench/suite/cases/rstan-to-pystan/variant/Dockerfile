FROM alexgshaw/rstan-to-pystan@sha256:b23d48839408524326095c15ada4145c772b87646cf299022ff1f58c97baba91
RUN sed -i -e 's|^URIs: http://archive.ubuntu.com/ubuntu/$|URIs: https://archive.ubuntu.com/ubuntu/|' -e 's|^URIs: http://security.ubuntu.com/ubuntu/$|URIs: https://security.ubuntu.com/ubuntu/|' /etc/apt/sources.list.d/ubuntu.sources
