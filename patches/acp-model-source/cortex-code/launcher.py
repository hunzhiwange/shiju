#!/usr/bin/python3 -I
"""Run CLI-local Cortex or its separate, sandboxed session-model composition."""
import asyncio
import json
import os
from pathlib import Path
import runpy
import stat
import sys
from urllib.parse import urlsplit

INSTALL = Path(__file__).resolve().parent
SANDBOX = """
(version 1)
(allow default)
(deny file-read-data (subpath "/Users") (subpath "/private/tmp")
    (subpath "/private/var/folders") (subpath (param "host-home")))
(allow file-read-data (subpath (param "workspace")) (subpath (param "state"))
    (subpath (param "install")) (literal "/dev/null"))
(deny file-write*)
(allow file-write* (subpath (param "write-root")) (subpath (param "state"))
    (literal "/dev/null"))
(deny file-read-data file-write* (subpath (param "git")) (subpath (param "env-file"))
    (subpath (param "project-settings")))
(deny process-fork process-exec)
(allow process-exec (literal (param "native")))
(deny network*)
(allow network-outbound (remote tcp (param "relay")))
"""


def main():
    reference = os.environ.get('VW_ACP_LAUNCH_REFERENCE')
    launch = os.environ.get('SHIJU_CORTEX_LAUNCH')
    if not reference and not launch:
        upstream = INSTALL / 'cortex-native'
        os.execv(upstream, [str(upstream), *sys.argv[1:]])
    if not reference or not launch or sys.argv[1:] != ['acp', 'serve']:
        raise SystemExit('Managed Cortex requires a private ACP launch configuration')

    config = Path(launch)
    meta = config.lstat()
    state = config.parent.resolve(strict=True)
    owner = os.geteuid()
    if (not stat.S_ISREG(meta.st_mode) or stat.S_IMODE(meta.st_mode) != 0o600
            or meta.st_uid != owner or state.stat().st_uid != owner
            or stat.S_IMODE(state.stat().st_mode) != 0o700):
        raise SystemExit('Insecure Cortex launch configuration')

    settings = json.loads(config.read_bytes())
    workspace = Path(settings['workspace']).resolve(strict=True)
    write_root = Path(settings['write_root']).resolve(strict=True)
    endpoint = urlsplit(settings['endpoint'])
    host_home = Path.home().resolve(strict=True)
    model = settings['model']
    if (not workspace.is_dir() or workspace == host_home or workspace in host_home.parents
            or not write_root.is_dir() or not write_root.is_relative_to(workspace)
            or not state.is_dir() or workspace.is_relative_to(state) or state.is_relative_to(workspace)
            or endpoint.scheme != 'http' or endpoint.hostname != '127.0.0.1'
            or not endpoint.port or endpoint.username or endpoint.password
            or endpoint.path != '/v1' or endpoint.query or endpoint.fragment
            or not model or model == 'auto' or not settings['token']):
        raise SystemExit('Invalid Cortex workspace or relay binding')
    native = INSTALL / 'cortex-local'
    os.chdir(workspace)
    params = {
        'install': INSTALL, 'native': native, 'state': state, 'workspace': workspace,
        'write-root': write_root, 'host-home': host_home, 'git': workspace / '.git',
        'env-file': workspace / '.env', 'project-settings': workspace / '.cortex',
        'relay': f'localhost:{endpoint.port}',
    }
    command = ['/usr/bin/sandbox-exec']
    for key, value in params.items():
        command += ['-D', f'{key}={value}']
    command += ['-p', SANDBOX, str(native), 'acp', 'serve', '--model', model,
                '--workdir', str(workspace), '--only-explicit-mcp-servers',
                '--only-explicit-plugins', '--only-explicit-skills']
    environment = {
        'PATH': '/usr/bin:/bin', 'HOME': str(state), 'TMPDIR': str(state),
        'SNOWFLAKE_HOME': str(state / 'snowflake'), 'NO_PROXY': '127.0.0.1,localhost',
        'SHIJU_CORTEX_LOCAL': '1', 'CORTEX_AGENT_MODEL': model,
        'CORTEX_MODEL_ENDPOINT': settings['endpoint'], 'CORTEX_MODEL_API_KEY': settings['token'],
    }
    bridge = runpy.run_path(str(INSTALL / 'mcp_bridge.py'))
    raise SystemExit(asyncio.run(bridge['run'](command, environment, workspace)))


if __name__ == '__main__':
    main()
