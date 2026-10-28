FROM vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf
RUN python3 -m pip install --no-cache-dir --index-url https://download.pytorch.org/whl/cpu --extra-index-url https://pypi.org/simple 'torch==2.6.0+cpu' 'torchvision==0.21.0+cpu' 'numpy==2.2.6' 'timm==1.0.19' 'opencv-python-headless==4.12.0.88'
COPY mobilesam.tar.gz mobile_sam.pt /tmp/sam-sources/
RUN echo 'c4b85466ae0a1826433d8290998d3b10704a2f4ba5da399e8c7d9cb46de7580e  /tmp/sam-sources/mobilesam.tar.gz' | sha256sum -c - && echo '6dbb90523a35330fedd7f1d3dfc66f995213d81b29a5ca8108dbcdd4e37d6c2f  /tmp/sam-sources/mobile_sam.pt' | sha256sum -c - && mkdir /tmp/sam-source && tar -xzf /tmp/sam-sources/mobilesam.tar.gz -C /tmp/sam-source --strip-components=1 && python3 -m pip install --no-deps --no-cache-dir /tmp/sam-source && mkdir /opt/mobile-sam && cp /tmp/sam-sources/mobile_sam.pt /opt/mobile-sam/mobile_sam.pt && rm -rf /tmp/sam-sources /tmp/sam-source
WORKDIR /app
