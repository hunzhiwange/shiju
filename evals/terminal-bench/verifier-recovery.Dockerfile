# Verifier-only dependency repair; never used for another model attempt.
ARG BASE_IMAGE=sha256:9ae90d8c0bf0ad00e72f9b07a65855357be9d2ac686014d7044e5cb8006d281c
FROM ${BASE_IMAGE}
COPY ca-certificates.crt /opt/verifier-recovery-ca.pem
RUN echo 'a3413a37a8e09cc21b2c11c9ffb23d92d2fc9d1933c9e7617f5c4fba4f72d37d  /opt/verifier-recovery-ca.pem' | sha256sum -c - \
 && sed -i 's|http://deb.debian.org|https://deb.debian.org|g' /etc/apt/sources.list.d/debian.sources \
 && printf 'Acquire::https::CaInfo "/opt/verifier-recovery-ca.pem";\nAcquire::Retries "5";\nAcquire::https::Timeout "30";\n' > /etc/apt/apt.conf.d/80-verifier-recovery \
 && apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y curl \
 && dpkg-query -W > /opt/verifier-recovery-packages.txt
ENV CURL_CA_BUNDLE=/opt/verifier-recovery-ca.pem SSL_CERT_FILE=/opt/verifier-recovery-ca.pem
