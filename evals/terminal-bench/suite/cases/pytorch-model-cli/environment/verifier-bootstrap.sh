#!/bin/sh
# Only the verifier phase receives these original installer/release caches.
set -eu
cd /opt/vw-verifier-deps
mkdir -p bin
cat > bin/curl <<'EOF'
#!/bin/sh
if [ "$#" = 2 ] && [ "$1" = '-LsSf' ] && [ "$2" = 'https://astral.sh/uv/0.9.5/install.sh' ]; then
    exec cat /opt/vw-verifier-deps/uv-installer.sh
fi
exec /usr/bin/curl "$@"
EOF
chmod 755 bin/curl
# Preserve the Debian distributions and package versions, using TLS transport.
python3 - <<'PY'
from pathlib import Path
for path in [Path('/etc/apt/sources.list'), *Path('/etc/apt/sources.list.d').glob('*.sources'), *Path('/etc/apt/sources.list.d').glob('*.list')]:
    if path.is_file():
        text = path.read_text()
        path.write_text(text.replace('http://deb.debian.org/', 'https://deb.debian.org/').replace('http://security.debian.org/', 'https://security.debian.org/'))
PY
printf '%s\n' 'Acquire::Retries "0";' 'Acquire::http::Timeout "15";' 'Acquire::https::Timeout "15";' > /etc/apt/apt.conf.d/99vw-verifier-network
# Supply the exact upstream dataset after the agent, before the original test.
# The checker still decodes MNIST and executes the candidate on 50 real images.
python3 - <<'PY'
from pathlib import Path
import gzip
import hashlib
import json
import tarfile

artifacts = [Path('/app') / name for name in ('cli_tool', 'weights.json', 'prediction.txt')]
before = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in artifacts if p.is_file()}
resources = {
    'train-images-idx3-ubyte.gz': 'f68b3c2dcbeaaa9fbdd348bbdeb94873',
    'train-labels-idx1-ubyte.gz': 'd53e105ee54ea40749a09fcbcd1e9432',
    't10k-images-idx3-ubyte.gz': '9fb629c4189551a2d022fa330f9573f3',
    't10k-labels-idx1-ubyte.gz': 'ec29112dd5afa0611ce80d1b7f02629c',
}
root = Path('/app/data/MNIST/raw')
root.mkdir(parents=True, exist_ok=True)
with tarfile.open('mnist.tar') as archive:
    assert {m.name for m in archive.getmembers()} == set(resources)
    for name, md5 in resources.items():
        content = archive.extractfile(name).read()
        assert hashlib.md5(content).hexdigest() == md5
        raw = gzip.decompress(content)
        assert len(raw) < 50 * 2**20
        target = root / name.removesuffix('.gz')
        if not target.exists():
            target.write_bytes(raw)
after = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in artifacts if p.is_file()}
assert after == before
print(json.dumps({'mnist_official_md5': resources, 'candidate_artifacts_unchanged': before}, sort_keys=True))
PY
