"""Explicitly approved ARM64 John build; canonical amd64 remains separate."""
import hashlib
import json
import re
import shlex

from suite.state import digest, load

PARENT = 'crack-7z-hash'
ID = 'crack-7z-hash-arm64-compat'
BASE = 'ubuntu@sha256:24737cef17f4e22b1977df7d9a05e77c4e28b7da5b8911f8d8ccda136bf3573c'
COMMIT = '8b5bfefbdc9b643513037f30c8200f5dc0ffc841'
TREE = 'ee4163c03a0f0792d2f3fb928751bc6a889e6029'
SOURCE = '08082dd15e6396f256598b452049a8ce996dc7d4478710cb8cb71645fb0b9a36'
PACKAGES = '6e46e1db616877b770db86e1777197004932dd702d3dccadf61a708821d739f0'
DEBS = 'bac937a8a08dc01b8d493de8253275811d46ff0c8ab7cdbca47f7d69a12ae1bd'
FILES = ('contract.json', 'Dockerfile', 'packages.lock', 'deb-lock.json', 'SHA256SUMS')
# Include all regular dpkg-owned files, including symlink targets, in stable order.
PACKAGE_BYTES_COMMAND = ("dpkg-query -W -f='${binary:Package}\\n' | xargs -r dpkg-query -L | LC_ALL=C sort -u | while IFS= read -r f; do "
                         "if [ -f \"$f\" ]; then printf '%s\\0' \"$f\"; fi; done | "
                         "xargs -0 -r sha256sum | sha256sum | cut -d' ' -f1")


def resolve(root, descriptor):
    if any((root / n).is_symlink() or not (root / n).is_file() for n in FILES):
        raise ValueError('missing trusted ARM64 variant files')
    c = load(root / 'contract.json')
    if (c['variant_id'] != ID or c['original_parent_id'] != PARENT
            or c['counts_as_original_official'] is not False
            or c['dependency_phase'] != 'image_build_before_agent'
            or c['architecture'] != 'aarch64' or c['john_commit'] != COMMIT
            or c['john_tree'] != TREE or c['source_tar_sha256'] != SOURCE
            or not re.fullmatch(r'vw-crack-7z-arm64-compat@sha256:[a-f0-9]{64}', c['image'])
            or any(not re.fullmatch('[a-f0-9]{64}', c[n]) for n in ('john_binary_sha256', 'package_files_sha256'))
            or digest(root / 'packages.lock') != PACKAGES or c['packages_lock_sha256'] != PACKAGES
            or digest(root / 'deb-lock.json') != DEBS or c['deb_lock_sha256'] != DEBS
            or digest(root / 'SHA256SUMS') != c['sha256sums_sha256']
            or digest(root / 'Dockerfile') != c['dockerfile_sha256']
            or [s.strip() for s in (root / 'Dockerfile').read_text().splitlines()
                if s.strip().upper().startswith('FROM ')] != ['FROM --platform=linux/arm64 ' + BASE]):
        raise ValueError('ARM64 variant exceeds approved source/package/architecture boundary')
    lock = load(root / 'deb-lock.json')['packages']
    sums = ''.join(p['sha256'] + '  ' + p['filename'] + '\n' for p in lock)
    if (root / 'SHA256SUMS').read_text() != sums:
        raise ValueError('ARM64 dependency archive hashes drift')
    value = {'id': ID, 'contract_sha256': digest(root / 'contract.json'), 'image': c['image'],
             'counts_as_original_official': False, 'files': {n: digest(root / n) for n in FILES}}
    if descriptor not in (value, {k: value[k] for k in ('id', 'contract_sha256')}):
        raise ValueError('ARM64 variant descriptor drift')
    return value


def expected_facts(root, contract):
    canonical = ''.join(sorted((root / 'packages.lock').read_text().splitlines(keepends=True)))
    return {'architecture': 'aarch64', 'john_commit': COMMIT, 'john_tree': TREE,
            'packages_sha256': hashlib.sha256(canonical.encode()).hexdigest(),
            'john_binary_sha256': contract['john_binary_sha256'],
            'package_files_sha256': contract['package_files_sha256']}


def probe_command(root, contract):
    # Ubuntu/John needs no Python. Keep the read-only proof shell-native too.
    fields = list(expected_facts(root, contract))
    fmt = json.dumps({n: '%s' for n in fields}) + '\n'
    script = ("set -euo pipefail; cd /app/john; git diff --exit-code HEAD -- . >/dev/null; "
              "test ! -e /app/solution.txt; "
              "test -z \"$(find /app/john -type f \\( -name '*.pot' -o -name '*.rec' -o -name 'john.log' \\) -print -quit)\"; "
              "architecture=$(uname -m); john_commit=$(git rev-parse HEAD); john_tree=$(git rev-parse HEAD^{tree}); "
              "packages_sha256=$(dpkg-query -W -f='${binary:Package}=${Version}\\n' | LC_ALL=C sort | sha256sum | cut -d' ' -f1); "
              "john_binary_sha256=$(sha256sum /app/john/run/john | cut -d' ' -f1); "
              'package_files_sha256=$(' + PACKAGE_BYTES_COMMAND + '); printf ' + shlex.quote(fmt) + ' ' +
              ' '.join('"$' + n + '"' for n in fields))
    return 'bash -c ' + shlex.quote(script)
