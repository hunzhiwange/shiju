FROM --platform=linux/arm64 ubuntu@sha256:24737cef17f4e22b1977df7d9a05e77c4e28b7da5b8911f8d8ccda136bf3573c
COPY native-pretest-package-versions.lock /opt/vw-compat/packages.lock
COPY debs/ /tmp/debs/
COPY SHA256SUMS /tmp/debs/SHA256SUMS
RUN cd /tmp/debs && sha256sum -c SHA256SUMS && DEBIAN_FRONTEND=noninteractive dpkg -i /tmp/debs/*.deb && dpkg-query -W -f='${binary:Package}=${Version}\n' | LC_ALL=C sort > /tmp/actual.lock && LC_ALL=C sort /opt/vw-compat/packages.lock > /tmp/expected.lock && diff -u /tmp/expected.lock /tmp/actual.lock
COPY apt-lists/ /var/lib/apt/lists/
COPY john-source-raw.tar /tmp/john-source-raw.tar
RUN echo '08082dd15e6396f256598b452049a8ce996dc7d4478710cb8cb71645fb0b9a36  /tmp/john-source-raw.tar' | sha256sum -c - && mkdir -p /app/john && tar -xf /tmp/john-source-raw.tar -C /app/john && rm /tmp/john-source-raw.tar
COPY public-john-git/ /app/john/.git/
RUN cd /app/john && git reset --hard 8b5bfefbdc9b643513037f30c8200f5dc0ffc841 && test "$(git rev-parse HEAD)" = 8b5bfefbdc9b643513037f30c8200f5dc0ffc841 && test "$(git rev-parse HEAD^{tree})" = ee4163c03a0f0792d2f3fb928751bc6a889e6029 && git diff --exit-code HEAD -- . && test -z "$(git remote)" && git tag -l | xargs -r git tag -d && git reflog expire --expire=now --all && git gc --prune=now --aggressive
WORKDIR /app/john/src
RUN ./configure --without-openssl && make
WORKDIR /app
COPY task-deps/ ./
RUN rm -rf /tmp/debs /tmp/actual.lock /tmp/expected.lock
