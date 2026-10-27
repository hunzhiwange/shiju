FROM alexgshaw/build-cython-ext@sha256:3612a38fadb89a96f74a1a951fb0b0af734198fd160571eeaba6401593234594
COPY planarity-0.6-cp313-cp313-linux_x86_64.whl /opt/vw-compat/
COPY constraints.txt /opt/vw-compat/constraints.txt
RUN echo "094b3f49d3226e903055ceb8770065fdf2a55404414c500d9530225057855ff6  /opt/vw-compat/planarity-0.6-cp313-cp313-linux_x86_64.whl" | sha256sum -c - \
    && python -m pip install --no-index --no-deps /opt/vw-compat/planarity-0.6-cp313-cp313-linux_x86_64.whl \
    && python -c 'import importlib.metadata as m; assert m.version("planarity")=="0.6"; assert m.version("numpy")=="2.3.0"'
ENV PIP_CONSTRAINT=/opt/vw-compat/constraints.txt
WORKDIR /app
