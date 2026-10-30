FROM vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf
COPY wheels/ /tmp/torch-wheels/
RUN python3 -m pip install --no-cache-dir --no-index --find-links=/tmp/torch-wheels torch==2.7.1+cpu && rm -rf /tmp/torch-wheels && python3 -c "import torch; assert torch.__version__ == '2.7.1+cpu'; print(torch.__version__)"
WORKDIR /app
