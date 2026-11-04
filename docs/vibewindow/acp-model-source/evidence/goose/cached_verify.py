#!/usr/bin/env python3
"""Run task 10's original Rust production entry from an already built binary.

Only Cargo startup is replaced. Model observations and success criteria remain
unchanged; cached results explicitly use binary_exit_code, never cargo_exit_code.
"""
import hashlib
import json
import os
from pathlib import Path
import sys

import verify as goose_verification

verification = goose_verification.verification
execute = verification.execute_cargo
slot = Path(verification.cargo_slot(os.environ))
binary = slot / 'verified-goose-production-tests'
if not binary.is_file() or not os.access(binary, os.X_OK):
    raise SystemExit('the validated production binary is unavailable in the owned slot')
actual = []

def run_binary(command, **kwargs):
    if command[:6] != ['cargo', 'test', '-p', 'vw-agent', '--lib', '--all-features'] or command[7] != '--':
        raise RuntimeError('unexpected production test command')
    actual[:] = [str(binary), command[6], *command[8:]]
    return execute(actual, **kwargs)

verification.execute_cargo = run_binary
if __name__ == '__main__':
    code = verification.main()
    output = Path(sys.argv[sys.argv.index('--output') + 1])
    report = json.loads(output.read_text())
    report['verification_kind'] = 'cached-production-binary'
    report['command'] = actual.copy()
    report['binary_exit_code'] = report.pop('cargo_exit_code', None)
    with binary.open('rb') as source:
        digest = hashlib.sha256()
        for block in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(block)
    report['binary_sha256'] = digest.hexdigest()
    output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'mode': report['mode'], 'passed': report['passed'], 'binary_exit_code': report['binary_exit_code'], 'effective_tests': report.get('effective_tests', 0), 'requests': len(report.get('requests', {}).get('requests', []))}))
    raise SystemExit(code)
