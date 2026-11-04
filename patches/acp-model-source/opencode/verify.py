#!/usr/bin/env python3
"""OpenCode-only recipe using the shared, unchanged production ACP acceptance entry."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import runpy
import sys
import tempfile

# macOS /var aliases /private/var; native child tools compare their real cwd.
tempfile.tempdir = str(Path(tempfile.gettempdir()).resolve())
ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--cli', required=True, type=Path)
parser.add_argument('--mode', required=True, choices=('session-model', 'cli-local'))
parser.add_argument('--output', required=True)
args = parser.parse_args()
cli = args.cli.resolve(strict=True)
# Digest extracted from the official npm native package after SHA512 integrity validation.
if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
    parser.error('only Darwin-arm64 has a verified binary digest')
if hashlib.sha256(cli.read_bytes()).hexdigest() != 'feaafead8cd563af0c2dcb386b0757e4355e327ebb6bd1aaad5440fe2f83596c':
    parser.error('CLI differs from the verified OpenCode 1.19.5 native binary')
manifest = json.loads((ROOT / 'docs/vibewindow/acp-model-source/cli-versions.json').read_text())
entry = next(row for row in manifest['entries'] if row['key'] == 'opencode')
if entry['distribution']['release'] != '1.19.5':
    parser.error('recipe requires the pinned OpenCode release')
entry['local_command_candidates'] = [str(cli)]
entry['installed']['command'] = [str(cli), '--version']
entry['acp_launch'] = [str(cli), 'acp']
entry['offline_acceptance'] = json.loads(Path(__file__).with_name('recipe.json').read_text())
sys.path.insert(0, str(ROOT / 'scripts'))
namespace = runpy.run_path(str(ROOT / 'scripts/verify_acp_model_source.py'))
globals_ = namespace['run'].__globals__
isolate = globals_['isolated_environment']
locate = globals_['locate']
fixture_type = globals_['ModelFixture']
successful = globals_['successful']
execute = globals_['execute_cargo']


def isolated(entry, directory, fixture, mode):
    environment = isolate(entry, directory, fixture, mode)
    for key in ('XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME', 'XDG_CACHE_HOME', 'OPENCODE_TEST_HOME'):
        path = directory / key.lower()
        path.mkdir(mode=0o700)
        environment[key] = str(path)
    if mode == 'session-model':
        # Both project JSON and discovered frontmatter must lose to the private route.
        (directory.parent / 'opencode.json').write_text(json.dumps({
            'model': 'forbidden/local', 'small_model': 'forbidden/small',
            'agent': {'build': {'model': 'forbidden/main'}, 'compaction': {'model': 'forbidden/compact'}}}))
        agents = directory.parent / '.opencode/agents'
        agents.mkdir(parents=True, mode=0o700)
        (agents / 'build.md').write_text('---\nmodel: forbidden/frontmatter\n---\nConflict fixture')
    return environment


def located(entry, environment):
    # Fixture-only network hygiene. These flags do not select a model or grant tools.
    environment.update(OPENCODE_DISABLE_MODELS_FETCH='1', OPENCODE_DISABLE_AUTOUPDATE='1')
    return locate(entry, environment)


class OpenCodeFixture(fixture_type):
    def __init__(self):
        super().__init__()
        fixture = self
        # Only model-service output is simulated. The real CLI must launch its task agent.
        def auxiliary_call(handler, body, messages):
            if any('AUXILIARY_CHILD' in json.dumps(message) for message in messages):
                return None
            for raw in body.get('tools', []):
                tool = raw.get('function', raw)
                if tool.get('name') == 'task':
                    return {'name': 'task', 'input': {
                        'description': 'Read offline challenge', 'subagent_type': 'general',
                        'prompt': 'AUXILIARY_CHILD: Read ' + fixture.read_path + ' twice using your read tool, then return its contents.'}}
            return None
        self.server.RequestHandlerClass.auxiliary_call = auxiliary_call


def require_auxiliary(evidence, flow):
    flow['requires_auxiliary'] = True
    return successful(evidence, flow)


def execute_with_diagnostics(command, *, env, timeout=1800):
    env = dict(env)
    path = Path(env['VW_ACP_ACCEPTANCE_FLOW']).with_name('diagnostics.json')
    path.write_text('{}')
    os.chmod(path, 0o600)
    env['VW_ACP_ACCEPTANCE_DIAGNOSTICS'] = str(path)
    return execute(command, env=env, timeout=timeout)


globals_.update(execute_cargo=execute_with_diagnostics, isolated_environment=isolated, locate=located, ModelFixture=OpenCodeFixture,
                successful=require_auxiliary)
with tempfile.TemporaryDirectory(prefix='opencode-recipe-') as directory:
    path = Path(directory) / 'manifest.json'
    path.write_text(json.dumps(manifest))
    sys.argv = ['verify_acp_model_source.py', '--agent', 'opencode', '--mode', args.mode,
                '--manifest', str(path), '--output', args.output, '--offline-fixture']
    raise SystemExit(namespace['main']())
