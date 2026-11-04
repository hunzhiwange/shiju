#!/usr/bin/env python3
"""Private Qwen configuration recipe; executes the shared production harness."""
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
entry = next(e for e in manifest['entries'] if e['key'] == 'qwen')
assert entry['distribution']['release'] == '0.26.7'
command = str(args.cli.absolute())
entry['local_command_candidates'] = [command]
entry['installed']['command'] = [command, '--version']
entry['acp_launch'] = [command, '--acp', '--experimental-skills', '--approval-mode', 'yolo']
entry['offline_acceptance'] = json.loads(Path(__file__).with_name('recipe.json').read_text())
# Qwen deduplicates reads of an unchanged file; use two challenge files.
sys.path.insert(0, str(root / 'scripts'))
with tempfile.TemporaryDirectory(prefix='qwen-recipe-') as directory:
    path = Path(directory) / 'manifest.json'
    path.write_text(json.dumps(manifest))
    sys.argv = ['verify_acp_model_source.py', '--agent', 'qwen', '--mode', args.mode,
                '--manifest', str(path), '--output', args.output, '--offline-fixture']
    namespace = runpy.run_path(str(root / 'scripts/verify_acp_model_source.py'))
    fixture_type = namespace['ModelFixture']
    class QwenFixture(fixture_type):
        def __setattr__(self, key, value):
            super().__setattr__(key, value)
            if key == 'read_path' and value and hasattr(self, 'challenge'):
                second = Path(value).with_name('challenge-second.txt')
                second.write_text(self.challenge)
                self.second_read_path = str(second)
    namespace['run'].__globals__['ModelFixture'] = QwenFixture
    execute = namespace['execute_cargo']
    def execute_with_diagnostics(command, *, env, timeout=1800):
        # Reuse the existing metadata-only ACP trace; never retain CLI payloads.
        import os
        env = dict(env)
        runtime = json.loads(Path(env['VW_ACP_ACCEPTANCE_MANIFEST']).read_text())
        if args.mode == 'session-model':
            project = Path(runtime['root']) / '.qwen'
            project.mkdir(mode=0o700)
            (project / 'settings.json').write_text(json.dumps({
                'security':{'auth':{'selectedType':'qwen-oauth','apiKey':'native-must-not-be-used'}},
                'model':{'name':'forbidden-native-model'},
                'modelProviders':{'openai':[{'id':'acp-fixture-primary','baseUrl':runtime['fixture']+'/v1','envKey':'FORBIDDEN_NATIVE_KEY'}]}}))
            (project / '.env').write_text('FORBIDDEN_NATIVE_KEY=native-must-not-be-used\n')
        diagnostic = Path(env['VW_ACP_ACCEPTANCE_FLOW']).with_name('diagnostics.json')
        diagnostic.write_text('{}')
        os.chmod(diagnostic, 0o600)
        env['VW_ACP_ACCEPTANCE_DIAGNOSTICS'] = str(diagnostic)
        return execute(command, env=env, timeout=timeout)
    namespace['run'].__globals__['execute_cargo'] = execute_with_diagnostics
    code = namespace['main']()
    raise SystemExit(code)
