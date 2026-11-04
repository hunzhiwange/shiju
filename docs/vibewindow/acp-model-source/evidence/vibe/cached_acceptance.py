#!/usr/bin/env python3
"""Replay the production acceptance entry in an owned slot's freshly built binary.

Only Cargo's launcher is replaced. Native CLI, production routing and all original
assertions/HTTP observations are unchanged. The report never claims a Cargo run.
"""
import argparse
import json
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / 'scripts'))
import verify_acp_model_source as harness

parser = argparse.ArgumentParser()
parser.add_argument('--binary', required=True, type=Path)
parser.add_argument('--mode', choices=('session-model', 'cli-local'), required=True)
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
slot = Path(harness.cargo_slot(os.environ))
binary = args.binary.resolve()
if not binary.is_relative_to(slot) or not binary.is_file() or not os.access(binary, os.X_OK):
    raise SystemExit('binary must belong to the currently owned Cargo slot')
for name in ('vibe.rs', 'vibe_tests.rs'):
    source = ROOT / 'crates/vw-agent/src/session/llm/acp/model_source/drivers' / name
    if binary.stat().st_mtime < source.stat().st_mtime:
        raise SystemExit('binary predates the current Vibe implementation')
execute = harness.execute_cargo
actual_command = []

def cached(command, *, env, timeout=300):
    global actual_command
    actual_command = [str(binary), command[6], *command[8:]]
    return execute(actual_command, env=env, timeout=timeout)

harness.execute_cargo = cached
os.environ['PATH'] = str(ROOT / 'target/vibe-cli-env/bin') + os.pathsep + os.environ['PATH']
result = harness.run(argparse.Namespace(agent='vibe', mode=args.mode,
    manifest=Path(__file__).with_name('manifest.json'), output=args.output, offline_fixture=True))
report = json.loads(args.output.read_text())
report['evidence_kind'] = 'cached-production-binary'
if 'cargo_exit_code' in report:
    report['binary_exit_code'] = report.pop('cargo_exit_code')
fingerprint = binary.parent.parent / '.fingerprint' / ('vw-agent-' + binary.name.removeprefix('vw_agent-')) / 'test-lib-vw_agent.json'
if fingerprint.is_file():
    metadata = json.loads(fingerprint.read_text())
    report['compiled_features'] = json.loads(metadata['features'])
report['command'] = actual_command
args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
print(json.dumps(report, ensure_ascii=False))
raise SystemExit(result)
