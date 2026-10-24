#!/usr/bin/env python3
"""Check formula version, binary URLs and hashes against the release manifest."""

import json
from pathlib import Path
import re
import sys

PACKAGES = ('shiju', 'shiju-acp', 'shiju-desktop', 'shiju-webview')
TARGETS = ('aarch64-apple-darwin', 'x86_64-apple-darwin',
           'aarch64-unknown-linux-gnu', 'x86_64-unknown-linux-gnu')


def verify(directory, tag):
    root = Path(directory)
    manifest = json.loads((root / 'dist-manifest.json').read_text())
    if manifest['announcement_tag'] != tag:
        raise SystemExit('Release manifest tag mismatch')
    for package in PACKAGES:
        body = (root / (package + '.rb')).read_text()
        if re.findall(r'^  version "([^"]+)"$', body, re.M) != [tag.removeprefix('v')]:
            raise SystemExit(f'Formula version mismatch: {package}')
        entries = re.findall(r'url "([^"\n]+)"\s+sha256 "([0-9a-f]{64})"', body)
        expected = {}
        for target in TARGETS:
            name = f'{package}-{target}.tar.gz'
            url = f'https://github.com/hunzhiwange/shiju/releases/download/{tag}/{name}'
            expected[url] = manifest['artifacts'][name]['checksums']['sha256']
        if len(entries) != len(expected) or dict(entries) != expected:
            raise SystemExit(f'Formula download URL or checksum mismatch: {package}')
        print(f'Verified formula version and four binary checksums: {package}')


if __name__ == '__main__':
    verify(*sys.argv[1:])
