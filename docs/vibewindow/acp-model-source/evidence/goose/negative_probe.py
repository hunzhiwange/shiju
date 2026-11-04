#!/usr/bin/env python3
"""Real Goose failure boundaries; fixture is only an offline model service."""
import argparse
import base64
import json
import os
from pathlib import Path
import tempfile
from native_probe import Client, Fixture

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', required=True, type=Path)
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
binary = args.binary.resolve()
report = {}
with tempfile.TemporaryDirectory(prefix='goose-negative-') as temporary:
    root = Path(temporary)
    fixture = Fixture(root)
    try:
        for case in ('provider-override', 'missing-credential'):
            runtime = root / case
            (runtime / 'config').mkdir(parents=True, mode=0o700)
            # A usable native secret exists; a missing managed credential must never borrow it.
            (runtime / 'config/secrets.yaml').write_text(json.dumps({'OPENAI_API_KEY': fixture.key}))
            env = {k: os.environ[k] for k in ('PATH', 'HOME', 'TMPDIR') if k in os.environ}
            env.update(GOOSE_PATH_ROOT=str(runtime), GOOSE_PROVIDER='openai', GOOSE_MODEL='acp-fixture-primary', SHIJU_GOOSE_MODEL_LOCK='acp-fixture-primary', OPENAI_HOST=fixture.url, OPENAI_API_KEY=fixture.key if case == 'provider-override' else '', OPENAI_BASE_PATH='v1/chat/completions', OPENAI_CUSTOM_HEADERS='', GOOSE_DISABLE_KEYRING='true')
            client = Client(binary, root, env, True)
            before = len(fixture.records)
            errors = fixture.errors
            rejected = False
            try:
                client.rpc('initialize', {'protocolVersion': 1, 'clientCapabilities': {}})
                params = {'cwd': str(root), 'mcpServers': []}
                if case == 'provider-override':
                    recipe = {'version': '1.0.0', 'title': 'Rejected provider', 'description': 'Offline negative', 'instructions': 'Reply briefly', 'settings': {'goose_provider': 'anthropic', 'goose_model': 'forbidden-model'}}
                    params['_meta'] = {'recipeDeeplink': base64.urlsafe_b64encode(json.dumps(recipe).encode()).decode().rstrip('=')}
                try:
                    session = client.rpc('session/new', params)
                    if case == 'missing-credential':
                        client.rpc('session/prompt', {'sessionId': session['sessionId'], 'prompt': [{'type': 'text', 'text': 'Reply briefly.'}]})
                except RuntimeError:
                    rejected = True
                report[case] = {'passed': (rejected if case == 'provider-override' else fixture.errors > errors) and len(fixture.records) == before,
                                'authorized_requests': len(fixture.records) - before,
                                'rejected_model_requests': fixture.errors - errors,
                                'rpc_rejected': rejected}
            finally:
                client.close()
    finally:
        fixture.close()
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
raise SystemExit(0 if all(r['passed'] for r in report.values()) else 1)
