#!/bin/sh
# Runs after the agent exits, in the original service environment.
# Preserve the image's Python/torch/transformers and the official test script.
set -eu
cd /opt/vw-verifier-deps
tar --no-same-owner -xf wheels.tar
# Configure only the verifier phase; the adapter restricts injected environment keys.
printf '[global]\nno-index = true\nfind-links = /opt/vw-verifier-deps/wheels\n' > /etc/pip.conf
python3 -m pip install --no-index --find-links=/opt/vw-verifier-deps/wheels pytest==8.4.1 psutil==7.0.0 requests==2.32.4 pytest-json-ctrf==0.3.5
python3 -c 'import pytest, psutil, requests; assert pytest.__version__ == "8.4.1"; assert psutil.__version__ == "7.0.0"; assert requests.__version__ == "2.32.4"'
