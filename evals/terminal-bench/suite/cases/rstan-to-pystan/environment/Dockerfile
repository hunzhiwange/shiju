# Build context: ignored diagnostics/rstan-to-pystan/tool-image-20261030c/context.
# Only SHA-bound public Python, wheels and Debian compiler packages enter this image.
FROM vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf
COPY public/ /opt/vw-public-deps/
RUN cd /opt/vw-public-deps && sha256sum -c SHA256SUMS && \
    mkdir /opt/clang && \
    for package in debs/*.deb; do dpkg-deb -x "$package" /opt/clang; done && \
    tar --exclude=python/share -xzf cpython312.tar.gz -C /opt && \
    /opt/python/bin/python3.12 -m pip install --no-index --no-cache-dir \
        --find-links wheels --target /opt/stan-site \
        pystan==3.10.0 httpstan==4.13.0 numpy==2.2.6 setuptools==70.3.0 && \
    rm -rf /opt/vw-public-deps /root/.cache
ENV PATH=/opt/python/bin:/opt/clang/usr/bin:/usr/local/bin:/usr/local/sbin:/usr/sbin:/usr/bin:/sbin:/bin \
    PYTHONPATH=/opt/stan-site \
    LD_LIBRARY_PATH=/opt/clang/usr/lib/x86_64-linux-gnu \
    CC=clang-14 CXX=clang++-14
# The frozen agent sets its own LD_LIBRARY_PATH. Register public Clang libraries
# with the system loader so tools also work in that actual execution path.
RUN printf '%s\n' /opt/clang/usr/lib/x86_64-linux-gnu > /etc/ld.so.conf.d/vw-clang.conf && ldconfig
WORKDIR /app
CMD ["python3"]
