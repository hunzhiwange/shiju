FROM python@sha256:fcf02c9e248b995ae2ca9aac6fa24b489f34b589dbfdc44698ad245d2ad41d1e
# The build context contains the pinned tokenizer and its Apache-2.0 license.
# Source revision and SHA256 are documented in README.md; no parent data enters.
RUN pip install --no-cache-dir tokenizers==0.22.2
COPY tokenizer.json LICENSE /opt/qwen/
WORKDIR /app
