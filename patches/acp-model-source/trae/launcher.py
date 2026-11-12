#!/usr/bin/env python3
"""A private TRAE native loop, with no access to the supplier's hosted services."""
import hashlib
import json
import os
from pathlib import Path
import stat
import re
import sys
from urllib.parse import urlsplit

NATIVE_SHA256 = '7648c66ffb535e83e4b1f5bbd820f0384d77234763cfb32700b1be3289358758'
LOCAL_SHA256 = 'f77806ca15efea1c33ffc4c3c08ab3f1c57111883dd3313822e673791484adde'


def private(path, directory=False):
    info = path.lstat()
    if info.st_uid != os.geteuid() or stat.S_IMODE(info.st_mode) != (0o700 if directory else 0o600):
        raise ValueError('Insecure TRAE private configuration')
    if not (stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode)):
        raise ValueError('Invalid TRAE private configuration')


def binding(path, reference):
    if not path.is_absolute():
        raise ValueError('Absolute TRAE binding required')
    private(path.parent, True)
    private(path)
    data = json.loads(path.read_text())
    if not re.fullmatch(r'vw-session-model:[0-9a-f]{64}', reference) or data['reference'] != reference:
        raise ValueError('TRAE session binding mismatch')
    url = urlsplit(data['endpoint'])
    if (url.scheme != 'http' or url.hostname != '127.0.0.1' or not url.port
            or url.path not in ('', '/') or url.username or url.password or url.query or url.fragment):
        raise ValueError('TRAE requires an exact owned loopback endpoint')
    if not data['model'] or data['output_limit'] <= 0 or data['context_limit'] <= 0:
        raise ValueError('Invalid TRAE selected model')
    return data, url.port


def policy(port):
    # Same sandbox covers native background clients and descendants. In particular,
    # no platform login, quota, telemetry, DNS, or unrelated loopback is reachable.
    return ('(version 1)(allow default)(deny network-outbound)'
            f'(allow network-outbound (remote tcp "localhost:{port}"))')


def main():
    root = Path(__file__).resolve().parent
    native = root / 'native/trae-cli'
    if hashlib.sha256(native.read_bytes()).hexdigest() != NATIVE_SHA256:
        raise ValueError('TRAE supplier executable checksum mismatch')
    path = os.environ.get('SHIJU_TRAE_MODEL_CONFIG')
    if not path:
        os.execv(str(native), [str(native), *sys.argv[1:]])
    if sys.argv[1:] != ['acp', 'serve']:
        raise ValueError('TRAE private entry requires native ACP arguments')
    data, port = binding(Path(path), os.environ.get('VW_ACP_LAUNCH_REFERENCE', ''))
    local = root / 'native/trae-local'
    if hashlib.sha256(local.read_bytes()).hexdigest() != LOCAL_SHA256:
        raise ValueError('TRAE local composition checksum mismatch')
    directory = Path(path).parent
    cache = directory / 'cache'
    cache.mkdir(mode=0o700)
    model = {'name': 'session-selected', 'context_window': data['context_limit'],
             'open_ai': {'model': data['model'], 'base_url': data['endpoint'].rstrip('/') + '/v1',
                         'api_key': '${VW_TRAE_RELAY_TOKEN}', 'max_tokens': data['output_limit']}}
    config = {'model': {'name': 'session-selected'}, 'models': [model],
              'disable_model_failover': True, 'vision_model_name': 'session-selected',
              'auto_compactor': {'model': 'session-selected'},
              'disable_auto_upgrade': True, 'enable_auto_memory': False}
    with os.fdopen(os.open(directory / 'trae_cli.yaml', os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), 'w') as f:
        json.dump(config, f)
    env = {key: value for key, value in os.environ.items()
           if not key.upper().startswith(('COCO_', 'TRAE_', 'OTEL_', 'VW_TRAE_'))}
    env.update(VW_TRAE_USER_DIR=str(directory), VW_TRAE_CACHE_DIR=str(cache),
               VW_TRAE_RELAY_TOKEN=os.environ['VW_TRAE_RELAY_TOKEN'],
               COCO_DISABLE_CLOUD_SESSION_UPLOAD='1')
    os.chdir(directory)
    # sandbox-exec compiles this policy before starting any native code.
    os.execve('/usr/bin/sandbox-exec', ['sandbox-exec', '-p', policy(port),
              str(local), 'acp', 'serve'], env)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, KeyError, OSError):
        raise SystemExit('TRAE private launch rejected') from None
