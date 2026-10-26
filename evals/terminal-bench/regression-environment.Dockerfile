# Environment repair only. Preserve the task/tests and signed Ubuntu archives.
ARG BASE_IMAGE=sha256:92a8a137a502cd8eb6aaf61bbd8c5fae597d9f027a29a55412a343f281901605
FROM ${BASE_IMAGE}
COPY ca-certificates.crt /opt/regression-ca.pem
RUN echo 'a3413a37a8e09cc21b2c11c9ffb23d92d2fc9d1933c9e7617f5c4fba4f72d37d  /opt/regression-ca.pem' | sha256sum -c -
RUN sed -i 's|http://archive.ubuntu.com|https://archive.ubuntu.com|g; s|http://security.ubuntu.com|https://security.ubuntu.com|g' /etc/apt/sources.list.d/ubuntu.sources \
 && printf 'Acquire::https::CaInfo "/opt/regression-ca.pem";\nAcquire::Retries "5";\nAcquire::http::Timeout "30";\nAcquire::https::Timeout "30";\n' > /etc/apt/apt.conf.d/80-regression-retries \
 && apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y curl \
 && dpkg-query -W > /opt/regression-packages.txt
ENV CURL_CA_BUNDLE=/opt/regression-ca.pem SSL_CERT_FILE=/opt/regression-ca.pem
