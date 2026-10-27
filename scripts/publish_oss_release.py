#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = ["oss2==2.19.1"]
# ///
"""Verify six Rust desktop installers; preview offline unless --publish is explicit."""

import argparse
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import re
import sys
import tempfile
import urllib.request

from prepare_oss_release import asset_specs, normalize_version, verify_file, CHUNK_SIZE

BUCKET = 'shiju-cloud'
PREFIX = 'agents/shiju-agent'
PUBLIC = 'https://www.shiju.cloud/'
LATEST = PREFIX + '/latest/manifest.json'
LONG_CACHE = 'public, max-age=31536000, immutable'
LATEST_CACHE = 'no-store, no-cache, max-age=0, must-revalidate'
MAX_MANIFEST = 1024 * 1024


class PublishError(ValueError):
    """A diagnostic containing only publisher-owned, non-sensitive text."""



class RecoverablePut(Exception):
    """Server no-clobber conflict or transport failure with unknown PUT outcome."""


def validate_manifest(manifest):
    if not isinstance(manifest, dict) or set(manifest) != {'schema_version', 'version', 'published_at', 'release_url', 'assets'}:
        raise PublishError('Invalid manifest fields; preview manifests are not publishable')
    version = manifest['version']
    if not isinstance(version, str) or normalize_version(version) != version:
        raise PublishError('Invalid stable version')
    if type(manifest['schema_version']) is not int or manifest['schema_version'] != 1:
        raise PublishError('Unsupported schema')
    if not isinstance(manifest['published_at'], str) or not manifest['published_at']:
        raise PublishError('Missing publication date')
    github = 'https://github.com/hunzhiwange/shiju/releases/'
    if manifest['release_url'] != github + 'tag/v' + version:
        raise PublishError('Unexpected release URL')
    assets = manifest['assets']
    if not isinstance(assets, list) or len(assets) != 6:
        raise PublishError('Exactly six installers required')
    for asset, (target, kind, name) in zip(assets, asset_specs(version)):
        if not isinstance(asset, dict) or set(asset) != {'name', 'target', 'kind', 'url', 'github_url', 'size', 'sha256'}:
            raise PublishError('Invalid asset fields')
        if (asset['name'], asset['target'], asset['kind']) != (name, target, kind):
            raise PublishError('Unexpected installer mapping')
        if asset['url'] != PUBLIC + PREFIX + f'/releases/v{version}/{name}' or asset['github_url'] != github + f'download/v{version}/{name}':
            raise PublishError('Unexpected installer URL')
        if type(asset['size']) is not int or not 0 < asset['size'] <= 5 * 1024**3:
            raise PublishError('Invalid installer size (single PUT limit: 5 GiB)')
        if not isinstance(asset['sha256'], str) or not re.fullmatch('[0-9a-f]{64}', asset['sha256']):
            raise PublishError('Invalid SHA-256')
    return manifest


def local_release(directory):
    directory = Path(directory)
    if not directory.is_absolute() or any(p.is_symlink() for p in (directory, *directory.parents)):
        raise PublishError('Use an absolute directory without symlink components')
    path = directory / 'manifest.json'
    if path.is_symlink() or not path.is_file() or path.stat().st_size > MAX_MANIFEST:
        raise PublishError('Missing or unsafe manifest.json; preview is insufficient')
    raw = path.read_bytes()
    manifest = validate_manifest(json.loads(raw))
    if directory.name != 'v' + manifest['version']:
        raise PublishError('Directory must match vX.Y.Z')
    for asset in manifest['assets']:
        if not verify_file(directory / asset['name'], asset):
            raise PublishError('Installer missing or content differs')
    return manifest, raw


def verify_stream(stream, size, sha):
    digest, count = hashlib.sha256(), 0
    while chunk := stream.read(CHUNK_SIZE):
        count += len(chunk)
        if count > size:
            raise PublishError('Remote object exceeds expected size')
        digest.update(chunk)
    if count != size or digest.hexdigest() != sha:
        raise PublishError('Remote object size or SHA-256 differs')


def verify_url(url, size, sha):
    request = urllib.request.Request(url, headers={'Cache-Control': 'no-cache'})
    with urllib.request.urlopen(request, timeout=30) as response:
        if response.status != 200:
            raise PublishError('Unexpected anonymous response')
        verify_stream(response, size, sha)


def load_credentials():
    path = Path.home() / '.config/shiju-oss-upload/config.json'
    config = json.loads(path.read_text(encoding='utf-8-sig')) if path.exists() else {}
    region = config.get('region', '')
    key = os.environ.get('OSS_ACCESS_KEY_ID') or config.get('access_key_id')
    secret = os.environ.get('OSS_ACCESS_KEY_SECRET') or config.get('access_key_secret')
    if not isinstance(region, str) or not re.fullmatch('[a-z]+-[a-z0-9-]+', region) or not key or not secret:
        raise PublishError('Missing local OSS region or credentials')
    return region, key, secret


class OssStore:
    def __init__(self, credentials):
        import oss2
        region, key, secret = credentials
        self.bucket = oss2.Bucket(oss2.AuthV4(key, secret), f'https://oss-{region}.aliyuncs.com',
                                  BUCKET, region=region, connect_timeout=30)

    def check_versioning(self):
        # Forbid-overwrite is ineffective for Enabled AND Suspended buckets.
        if self.bucket.get_bucket_versioning().status is not None:
            raise PublishError('Bucket versioning must never have been enabled')

    def read(self, key):
        try:
            with contextlib.closing(self.bucket.get_object(key)) as stream:
                data = stream.read(MAX_MANIFEST + 1)
                if len(data) > MAX_MANIFEST:
                    raise PublishError('Remote manifest too large')
                return data
        except Exception as error:
            if getattr(error, 'status', None) == 404 and getattr(error, 'code', None) == 'NoSuchKey':
                return None
            raise

    def create(self, key, stream, cache):
        import oss2
        try:
            self.bucket.put_object(key, stream, headers={'x-oss-forbid-overwrite': 'true',
                'Cache-Control': cache, 'Content-Type': 'application/json' if key.endswith('.json') else 'application/octet-stream'})
        except oss2.exceptions.RequestError as error:
            raise RecoverablePut() from error
        except oss2.exceptions.ServerError as error:
            if error.status == 409 and error.code == 'FileAlreadyExists':
                raise RecoverablePut() from error
            raise

    def verify(self, key, size, sha):
        with contextlib.closing(self.bucket.get_object(key)) as stream:
            verify_stream(stream, size, sha)


def receipt_path(directory):
    return directory / 'oss-publish-receipt.json'


def receipt(directory, manifest, raw, stage):
    data = dict(version=manifest['version'], target=f'oss://{BUCKET}/{LATEST}',
                sha256=hashlib.sha256(raw).hexdigest(), stage=stage,
                assets=[dict(url=a['url'], sha256=a['sha256'], size=a['size']) for a in manifest['assets']])
    fd, name = tempfile.mkstemp(prefix='.receipt-', dir=directory)
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(data, stream, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(name, receipt_path(directory))
    finally:
        Path(name).unlink(missing_ok=True)


def ensure_object(store, key, stream, size, sha, cache):
    try:
        store.create(key, stream, cache)
    except RecoverablePut:
        # Conflict or unknown PUT outcome: a full origin read is the only recovery.
        store.verify(key, size, sha)
        return
    store.verify(key, size, sha)


def publish_release(directory, store, public_verify=None):
    directory = Path(directory)
    manifest, raw = local_release(directory)
    public_verify = public_verify or (lambda key, size, sha: verify_url(PUBLIC + key, size, sha))
    store.check_versioning()
    remote = store.read(LATEST)
    if remote is not None:
        previous = validate_manifest(json.loads(remote))
        previous_version = tuple(map(int, previous['version'].split('.')))
        version = tuple(map(int, manifest['version'].split('.')))
        if previous_version > version:
            raise PublishError('Downgrade refused')
        if previous_version == version and remote != raw:
            raise PublishError('Same version has different manifest bytes')
        if previous_version < version:
            raise PublishError('Cannot advance existing latest: OSS PutObject has no supported destination CAS')
    receipt(directory, manifest, raw, 'uploading_versions')
    prefix = PREFIX + '/releases/v' + manifest['version'] + '/'
    objects = []
    for asset in manifest['assets']:
        key = prefix + asset['name']
        with (directory / asset['name']).open('rb') as stream:
            ensure_object(store, key, stream, asset['size'], asset['sha256'], LONG_CACHE)
        objects.append((key, asset['size'], asset['sha256']))
    sha = hashlib.sha256(raw).hexdigest()
    ensure_object(store, prefix + 'manifest.json', io.BytesIO(raw), len(raw), sha, LONG_CACHE)
    objects.append((prefix + 'manifest.json', len(raw), sha))
    for key, size, digest in objects:
        public_verify(key, size, digest)
    receipt(directory, manifest, raw, 'publishing_latest')
    # Both first creation and resume use server-side no-clobber, never overwrite.
    ensure_object(store, LATEST, io.BytesIO(raw), len(raw), sha, LATEST_CACHE)
    public_verify(LATEST, len(raw), sha)
    receipt(directory, manifest, raw, 'complete')
    return receipt_path(directory)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dir', required=True, type=Path, help='Absolute prepared vX.Y.Z directory')
    parser.add_argument('--publish', action='store_true', help='Write OSS; existing latest upgrades fail closed without CAS')
    args = parser.parse_args(argv)
    try:
        manifest, _ = local_release(args.dir)
        for asset in manifest['assets']:
            print(f'{asset["target"]}: {asset["url"]} ({asset["size"]} bytes, {asset["sha256"]})')
        if args.publish:
            print(publish_release(args.dir, OssStore(load_credentials())))
        else:
            print('Offline preview only. Existing latest cannot be advanced without supported OSS CAS.')
    except PublishError as error:
        print(f'Publish failed: {error}', file=sys.stderr)
        return 1
    except Exception:
        # SDK and malformed input exceptions can contain secrets or raw responses.
        print('Publish failed; check local inputs, OSS access/versioning, object conflicts and CAS limitation. '
              'Inspect receipt and remote state before retry; no automatic overwrite.', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
