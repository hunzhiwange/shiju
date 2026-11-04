"""Codex-native entry using task 10's process/version/config/slot harness unchanged.

Only the upstream fixture, vendor-specific Rust test selectors and protocol
predicate differ. The Rust entries call the real production ACP request path.
This is separate evidence from the task 10 shared Chat/Messages baseline.
"""
from pathlib import Path
import sys
import argparse
import os
import json
import hashlib
import re
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import verify_acp_model_source as verifier
from responses_fixture import ResponsesFixture


def successful(evidence, flow):
    records = evidence.get('requests', [])
    models = {record.get('model') for record in records}
    expected = ({'acp-fixture-primary'} if flow.get('mode') == 'cli-local'
                else {'acp-fixture-primary', 'acp-fixture-switched'})
    return (evidence.get('errors') == 0 and len(records) >= 4 and models == expected
            and all(record.get('authenticated') is True
                    and record.get('route') == '/v1/responses' for record in records)
            and any(record.get('tool_results', 0) >= 2 for record in records)
            and all(flow.get(key) is True for key in
                    ('new', 'second_turn', 'cancel', 'restart', 'switch', 'isolated')))


original_locate = verifier.locate
verified_identity = {}

def pinned_locate(entry, env):
    if entry.get('key') != 'codex':
        raise verifier.Unavailable('this private entry verifies Codex only')
    config = original_locate(entry, env)
    bundle = Path(config['command']).resolve()
    pin = entry.get('model_source_patch', {})
    if hashlib.sha256(bundle.read_bytes()).hexdigest() != pin.get('bundle_sha256'):
        raise verifier.VersionMismatch('installed Codex adapter patch differs from pinned bundle')
    marker = subprocess.run([config['command'], '--shiju-model-source-check'], env=env,
                            capture_output=True, timeout=20)
    if marker.returncode or marker.stdout.decode().strip() != pin.get('marker'):
        raise verifier.VersionMismatch('installed Codex model-source marker differs')
    node = shutil.which('node', path=env.get('PATH'))
    if not node:
        raise verifier.Unavailable('Codex adapter requires installed Node')
    program = "const {createRequire}=require('node:module'); console.log(createRequire(process.argv[1]).resolve('@openai/codex/bin/codex.js'));"
    resolution = subprocess.run([node, '-e', program, str(bundle)], env=env,
                                capture_output=True, timeout=20)
    if resolution.returncode:
        raise verifier.Unavailable('Codex adapter engine resolution failed')
    engine = Path(resolution.stdout.decode().strip())
    package = json.loads((engine.parent.parent / 'package.json').read_text())
    expected = entry['underlying']['release']
    if package.get('name') != '@openai/codex' or package.get('version') != expected:
        raise verifier.VersionMismatch('adapter-resolved Codex engine package differs')
    version = subprocess.run([node, str(engine), '--version'], env=env,
                             capture_output=True, timeout=20)
    if version.returncode or not re.search(r'(?<![\w.])' + re.escape(expected) + r'(?![\w.])', version.stdout.decode()):
        raise verifier.VersionMismatch('adapter-resolved Codex engine version differs')
    verified_identity.update(adapter=entry['distribution']['release'], engine=expected,
                             bundle_sha256=pin['bundle_sha256'])
    return config


verifier.locate = pinned_locate
verifier.ModelFixture = ResponsesFixture
verifier.successful = successful
verifier.TESTS = {
    'session-model': 'acp_model_source_codex_real_cli_session_model',
    'cli-local': 'acp_model_source_codex_real_cli_cli_local',
}
if __name__ == '__main__':
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument('--cached-test-binary', type=Path)
    local, remaining = parser.parse_known_args()
    sys.argv = [sys.argv[0], *remaining]
    executed = []
    diagnostics = []
    if local.cached_test_binary:
        binary = local.cached_test_binary.resolve()
        target = Path(os.environ.get('CARGO_TARGET_DIR', '')).resolve()
        if binary.parent != target / 'debug/deps' or not binary.name.startswith('vw_agent-') or not binary.is_file() or not os.access(binary, os.X_OK):
            raise SystemExit('cached binary must be the verified test artifact in the held Cargo slot')
        execute = verifier.execute_cargo
        def run_binary(command, *, env, timeout=300):
            actual = [str(binary), command[6], *command[8:]]
            executed[:] = actual
            result = execute(actual, env=env, timeout=timeout)
            path = Path(env['VW_ACP_ACCEPTANCE_FLOW']).with_suffix('.codex-diag.json')
            if path.exists():
                raw = json.loads(path.read_text())
                keys = ('result_ok', 'done', 'has_output', 'fixture_reply', 'http_401',
                        'http_403', 'http_422', 'transport_metadata', 'unsupported_transport',
                        'missing_credentials', 'stale_state', 'profile_reissue', 'initialize_error', 'aborted')
                diagnostics[:] = [{key: item.get(key) is True for key in keys} for item in raw]
            return result
        verifier.execute_cargo = run_binary
    code = verifier.main()
    if local.cached_test_binary and '--output' in remaining:
        output = Path(remaining[remaining.index('--output') + 1])
        report = json.loads(output.read_text())
        report['identity'] = verified_identity
        report['execution_kind'] = 'cached-production-binary'
        if 'cargo_exit_code' in report:
            report['binary_exit_code'] = report.pop('cargo_exit_code')
        report['command'] = executed
        report['native_diagnostics'] = diagnostics
        output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    raise SystemExit(code)
