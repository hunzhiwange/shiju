"""Run verified OMP and its explicit MCP mounts on one private loopback port."""
import asyncio
import json
import os
from pathlib import Path
import runpy
import sys
import tempfile
from urllib.parse import urlsplit


def write_private(path, contents):
    with tempfile.NamedTemporaryFile(dir=path.parent, mode='w', delete=False) as stream:
        temporary = Path(stream.name)
        try:
            stream.write(contents)
            stream.flush()
            temporary.replace(path)
        finally:
            temporary.unlink(missing_ok=True)


def main():
    policy, executable, *args = sys.argv[1:]
    root = Path(policy).resolve(strict=True).parent
    home = Path(os.environ['HOME'])
    agent = Path(os.environ['PI_CODING_AGENT_DIR'])
    workspace = Path.cwd()
    models = agent / 'models.yml'
    registry = json.loads((root / 'omp-models-source.json').read_text())
    provider = registry['providers']['shiju-session']
    endpoint = urlsplit(provider['baseUrl'])
    original = f'localhost:{endpoint.port}'
    sandbox = Path(policy).read_text()
    if (endpoint.scheme != 'http' or endpoint.netloc != f'127.0.0.1:{endpoint.port}'
            or not endpoint.port or endpoint.path != '/v1' or endpoint.query or endpoint.fragment
            or sandbox.count(original) != 1):
        raise ValueError('Invalid OMP model or sandbox binding')
    token = os.environ['SHIJU_OMP_RELAY_TOKEN']
    settings = agent / 'shiju-mcp.yml'
    write_private(settings, 'tools:\n  xdev: false\n')

    def configure(port):
        provider['baseUrl'] = f'http://127.0.0.1:{port}/v1'
        write_private(models, json.dumps(registry))

    def command(port):
        return ['/usr/bin/sandbox-exec', '-p', sandbox.replace(original, f'localhost:{port}'),
                executable, *args, '--config', str(settings)]

    bridge = runpy.run_path(str(root / 'omp-mcp-bridge.py'))
    stdio = runpy.run_path(str(root / 'omp-mcp-stdio.py'))
    env = {
        'PATH': '/usr/bin:/bin', 'HOME': str(home), 'TMPDIR': str(home),
        'PI_CODING_AGENT_DIR': str(agent), 'NO_PROXY': '127.0.0.1,localhost',
        'SHIJU_OMP_RELAY_TOKEN': token,
    }
    raise SystemExit(asyncio.run(bridge['run'](
        command, env, workspace, endpoint, token, configure, stdio['Stdio'], stdio['allowed_tool'],
    )))


if __name__ == '__main__':
    main()
