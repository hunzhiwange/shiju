#!/usr/bin/env python3
"""Prepare eight unchanged GitHub installers for a separate, explicit OSS publish."""

import argparse
import hashlib
import http.client
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request

REPOSITORY = 'hunzhiwange/shiju'
PUBLIC_ROOT = 'https://www.shiju.cloud/agents/shiju-agent'
TIMEOUT = 30
ATTEMPTS = 3
CHUNK_SIZE = 1024 * 1024


def normalize_version(value):
    if not re.fullmatch(r'v?(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', value):
        raise ValueError('Version must be a stable X.Y.Z or vX.Y.Z (no leading zeroes)')
    return value.removeprefix('v')


def asset_specs(version):
    return [
        ('aarch64-unknown-linux-gnu', 'deb', f'shiju_{version}_arm64.deb'),
        ('x86_64-unknown-linux-gnu', 'deb', f'shiju_{version}_amd64.deb'),
        ('aarch64-unknown-linux-gnu', 'rpm', f'shiju-{version}-1.aarch64.rpm'),
        ('x86_64-unknown-linux-gnu', 'rpm', f'shiju-{version}-1.x86_64.rpm'),
        ('aarch64-pc-windows-msvc', 'msi', 'ShiJu-aarch64-pc-windows-msvc-dist.msi'),
        ('x86_64-pc-windows-msvc', 'msi', 'ShiJu-x86_64-pc-windows-msvc-dist.msi'),
        ('aarch64-apple-darwin', 'dmg', 'ShiJu-macos-aarch64-apple-darwin-dist.dmg'),
        ('x86_64-apple-darwin', 'dmg', 'ShiJu-macos-x86_64-apple-darwin-dist.dmg'),
    ]


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError('GitHub API redirect refused to protect authorization')


def github_token():
    for key in ('GITHUB_TOKEN', 'GH_TOKEN'):
        token = os.environ.get(key, '').strip()
        if token:
            return token
    try:
        result = subprocess.run(
            ['gh', 'auth', 'token', '--hostname', 'github.com'],
            capture_output=True, text=True, timeout=TIMEOUT, check=False)
    except (OSError, subprocess.SubprocessError):
        return None
    return result.stdout.strip() if result.returncode == 0 else None


def fetch_release(version):
    url = f'https://api.github.com/repos/{REPOSITORY}/releases/tags/v{version}'
    headers = {'Accept': 'application/vnd.github+json', 'User-Agent': 'vibewindow-release-preparer'}
    token = github_token()
    if token:
        headers['Authorization'] = 'Bearer ' + token
    opener = urllib.request.build_opener(NoRedirect())
    for attempt in range(ATTEMPTS):
        try:
            with opener.open(urllib.request.Request(url, headers=headers), timeout=TIMEOUT) as response:
                return json.load(response)
        except (OSError, http.client.HTTPException) as error:
            message = 'GitHub metadata request failed; check network, proxy and TLS certificates'
            if isinstance(error, urllib.error.HTTPError):
                message = f'GitHub metadata request failed (HTTP {error.code})'
                if error.code in (403, 429) and error.headers.get('X-RateLimit-Remaining') == '0':
                    message += '; API rate limit exhausted; set GITHUB_TOKEN/GH_TOKEN or run gh auth login --hostname github.com, or wait for the quota to reset'
                elif error.code == 401:
                    message += '; credentials rejected; check GITHUB_TOKEN/GH_TOKEN or gh authentication'
                elif error.code == 404:
                    message += f'; verify published tag v{version} and repository access'
                elif error.code == 403:
                    message += '; access forbidden; check credential permissions and GitHub API limits'
                retryable = error.code == 429 or 500 <= error.code < 600
                error.close()
                if not retryable:
                    raise ValueError(message) from error
            if attempt == ATTEMPTS - 1:
                raise ValueError(message) from error
            time.sleep(attempt + 1)


def build_manifest(release, version):
    version = normalize_version(version)
    release_url = f'https://github.com/{REPOSITORY}/releases/tag/v{version}'
    if (release.get('tag_name') != f'v{version}' or release.get('draft') is not False
            or release.get('prerelease') is not False or release.get('html_url') != release_url):
        raise ValueError('Release must match the requested stable, published GitHub tag')
    published_at = release.get('published_at')
    if not isinstance(published_at, str) or not published_at:
        raise ValueError('Release published_at is missing')
    raw_assets = release.get('assets')
    if not isinstance(raw_assets, list):
        raise ValueError('Release assets are missing')
    assets = []
    for target, kind, name in asset_specs(version):
        matches = [a for a in raw_assets if a.get('name') == name]
        if len(matches) != 1:
            raise ValueError(f'Expected exactly one release asset: {name}')
        asset = matches[0]
        digest, size = asset.get('digest'), asset.get('size')
        github_url = f'https://github.com/{REPOSITORY}/releases/download/v{version}/{name}'
        if not isinstance(digest, str) or not re.fullmatch(r'sha256:[0-9a-f]{64}', digest):
            raise ValueError(f'Missing or invalid GitHub SHA-256 digest: {name}')
        if type(size) is not int or size <= 0:
            raise ValueError(f'Invalid byte size: {name}')
        if asset.get('browser_download_url') != github_url:
            raise ValueError(f'Unexpected download URL: {name}')
        assets.append(dict(name=name, target=target, kind=kind,
                           url=f'{PUBLIC_ROOT}/releases/v{version}/{name}',
                           github_url=github_url, size=size, sha256=digest[7:]))
    return dict(schema_version=1, version=version, published_at=published_at,
                release_url=release_url, assets=assets)


def verify_file(path, asset):
    if path.is_symlink() or not path.is_file() or path.stat().st_size != asset['size']:
        return False
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        while chunk := stream.read(CHUNK_SIZE):
            digest.update(chunk)
    return digest.hexdigest() == asset['sha256']


def download_asset(asset, directory):
    destination = directory / asset['name']
    if destination.exists() or destination.is_symlink():
        if verify_file(destination, asset):
            return
        raise ValueError(f'Existing file differs: {destination}; move it aside and retry')
    temporary = destination.with_name(destination.name + '.part')
    if temporary.is_symlink():
        raise ValueError(f'Refusing symlink: {temporary}')
    for attempt in range(ATTEMPTS):
        try:
            # Never attach API credentials to installer requests or their redirects.
            with urllib.request.urlopen(asset['github_url'], timeout=TIMEOUT) as response, temporary.open('wb') as output:
                size = 0
                while chunk := response.read(CHUNK_SIZE):
                    size += len(chunk)
                    if size > asset['size']:
                        raise ValueError('Download exceeds expected size')
                    output.write(chunk)
            if not verify_file(temporary, asset):
                raise ValueError('Downloaded size or SHA-256 differs')
            # Hard link provides atomic, no-clobber publication on the same filesystem.
            os.link(temporary, destination)
            temporary.unlink()
            return
        except (OSError, http.client.HTTPException, ValueError) as error:
            if isinstance(error, urllib.error.HTTPError):
                error.close()
            temporary.unlink(missing_ok=True)
            if attempt == ATTEMPTS - 1:
                raise ValueError(f'Download failed for {asset["name"]}; rerun to reuse verified files') from error
            time.sleep(attempt + 1)


def write_manifest(path, manifest):
    temporary = path.with_name(path.name + '.part')
    if temporary.is_symlink():
        raise ValueError(f'Refusing symlink: {temporary}')
    temporary.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    temporary.replace(path)


def prepare_release(version, output_dir, metadata_only=False):
    version = normalize_version(version)
    manifest = build_manifest(fetch_release(version), version)
    directory = Path(output_dir) / f'v{version}'
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / ('manifest.preview.json' if metadata_only else 'manifest.json')
    if not metadata_only:
        # Invalidate readiness before checking/recovering an existing directory.
        path.unlink(missing_ok=True)
    for asset in manifest['assets']:
        print(f'{asset["name"]}: {asset["size"]} bytes\n  {asset["github_url"]}\n  -> {asset["url"]}')
        if not metadata_only:
            download_asset(asset, directory)
    write_manifest(path, manifest)
    print(f'{"Candidate only (no installers downloaded)" if metadata_only else "Verified release"}: {path}')
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version', required=True, help='Explicit stable X.Y.Z or vX.Y.Z')
    parser.add_argument('--output-dir', required=True, type=Path, help='Absolute local output directory')
    parser.add_argument('--metadata-only', action='store_true', help='Write manifest.preview.json without downloading installers')
    args = parser.parse_args()
    if not args.output_dir.is_absolute():
        parser.error('--output-dir must be absolute')
    try:
        prepare_release(args.version, args.output_dir, args.metadata_only)
    except (ValueError, OSError) as error:
        print(f'Error: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
