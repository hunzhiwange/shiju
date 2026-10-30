FROM alexgshaw/mcmc-sampling-stan@sha256:073fc36a3347b4f6b43bd6e98339542c41ed531c771af8bee40ba10f00cfabaf
COPY cmake-debs/ /opt/vw-cmake-debs/
COPY SHA256SUMS /opt/vw-cmake-debs/SHA256SUMS
RUN cd /opt/vw-cmake-debs \
 && sha256sum -c SHA256SUMS \
 && dpkg -i ./*.deb \
 && cmake --version \
 && rm -rf /opt/vw-cmake-debs
LABEL org.opencontainers.image.title="mcmc-sampling-stan authorized CMake compatibility variant"
LABEL vw.original-parent="mcmc-sampling-stan" vw.counts-as-original-official="false"
