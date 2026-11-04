#!/usr/bin/env python3
"""Private iFlow configuration recipe; executes the shared production harness."""
import argparse
import json
from pathlib import Path
import runpy
import sys
import tempfile

root = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser()
parser.add_argument('--cli', required=True, type=Path)
parser.add_argument('--mode', required=True, choices=['session-model', 'cli-local'])
parser.add_argument('--output', required=True)
args = parser.parse_args()
manifest = json.loads((root / 'docs/vibewindow/acp-model-source/cli-versions.json').read_text())
entry = next(e for e in manifest['entries'] if e['key'] == 'iflow')
assert entry['distribution']['release'] == '0.5.19'
command = str(args.cli.absolute())
entry['local_command_candidates'] = [command]
entry['installed']['command'] = [command, '--version']
entry['acp_launch'] = [command, '--experimental-acp']
entry['offline_acceptance'] = json.loads(Path(__file__).with_name('recipe.json').read_text())
# The shared fixture recognizes iFlow read_file(absolute_path); use it unchanged.
sys.path.insert(0, str(root / 'scripts'))
with tempfile.TemporaryDirectory(prefix='iflow-recipe-') as directory:
    path = Path(directory) / 'manifest.json'
    path.write_text(json.dumps(manifest))
    sys.argv = ['verify_acp_model_source.py', '--agent', 'iflow', '--mode', args.mode,
                '--manifest', str(path), '--output', args.output, '--offline-fixture']
    namespace = runpy.run_path(str(root / 'scripts/verify_acp_model_source.py'))
    execute = namespace['execute_cargo']
    def execute_with_diagnostics(command, *, env, timeout=1800):
        # Reuse the existing metadata-only ACP trace; never retain CLI payloads.
        import os
        env = dict(env)
        diagnostic = Path(env['VW_ACP_ACCEPTANCE_FLOW']).with_name('diagnostics.json')
        diagnostic.write_text('{}')
        os.chmod(diagnostic, 0o600)
        env['VW_ACP_ACCEPTANCE_DIAGNOSTICS'] = str(diagnostic)
        return execute(command, env=env, timeout=timeout)
    namespace['run'].__globals__['execute_cargo'] = execute_with_diagnostics
    code = namespace['main']()
    raise SystemExit(code)
