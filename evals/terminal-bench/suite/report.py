"""Read-only report over a frozen catalog and contract JSONL records.

Run with python -m suite.report --manifest suite/catalog.json --results history.jsonl.
Default acceptance requires 89 scored parents and 356 scored children. --audit-only
permits incomplete coverage; --require-parents / --require-children restore the
respective gate even in audit mode. Invalid or duplicate evidence always fails.
A recovered checker result must replace the input snapshot of the same attempt,
never appear as a second attempt. This tool neither modifies history nor retries.
Diagnostic IDs follow Harbor package names: <parent>-m1 through <parent>-m4.
Calibration is deliberately absent from the experiment record contract and is
reported as unavailable, never inferred from readiness or official scores.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from suite.contract import validate_record


def expected_cases(manifest):
    """Use the complete frozen 89-task catalog, never observed-result denominators."""
    tasks = manifest['tasks']
    ids = [task['id'] for task in tasks]
    if (manifest.get('schema_version') != 1 or manifest.get('task_count') != 89
            or len(ids) != 89 or len(set(ids)) != 89
            or any(not isinstance(name, str) or not name
                   for name in ids)):
        raise ValueError('expected a complete unique 89-task manifest')
    return {'official': set(ids),
            'diagnostic': {f'{name}-m{index}' for name in ids for index in range(1, 5)}}


def summarize(records, expected=None):
    scored = [row for row in records if row['score_status'] == 'scored']
    count = len(records)
    return {
        'expected': expected,
        'observed': count,
        'missing': expected - count if expected is not None else None,
        'scored': len(scored),
        'unscored': count - len(scored),
        'coverage': len(scored) / expected if expected is not None else None,
        'passed': sum(row['reward'] == 1 for row in scored),
        'zero_reward': sum(row['reward'] == 0 for row in scored),
        'capability_failed': sum(row['failure_class'] == 'capability' for row in records),
        'infrastructure_failed': sum(row['failure_class'] == 'infrastructure' for row in records),
        'normal_terminal': sum(row['status'] == 'completed' for row in records),
        'truncated': sum(row['output_truncated'] or row['finish_reason'] == 'length'
                         for row in records),
        'finish_reasons': dict(sorted(Counter(row['finish_reason'] or 'unknown'
                                             for row in records).items())),
        'mean_reward': sum(row['reward'] for row in scored) / len(scored) if scored else None,
    }


def build_report(manifest, records):
    """Validate every row before aggregation; do not select the best/latest attempt."""
    expected = expected_cases(manifest)
    rows = list(records)
    attempts, cases, experiments = set(), set(), set()
    for row in rows:
        validate_record(row)
        key = row['kind'], row['case_id']
        if row['attempt_id'] in attempts or key in cases:
            raise ValueError('duplicate attempt or case evidence')
        if row['kind'] in expected and row['case_id'] not in expected[row['kind']]:
            raise ValueError('case outside frozen manifest')
        if row['kind'] == 'compatibility_variant' and row['case_id'] not in expected['official']:
            raise ValueError('variant parent outside frozen manifest')
        attempts.add(row['attempt_id'])
        cases.add(key)
        experiments.add(row['experiment_id'])
    if len(experiments) > 1:
        raise ValueError('mixed experiments forbidden')
    layers = {kind: summarize([row for row in rows if row['kind'] == kind], len(ids))
              for kind, ids in expected.items()}
    layers['compatibility_variant'] = summarize([row for row in rows if row['kind'] == 'compatibility_variant'])
    layers['readiness'] = summarize([row for row in rows if row['kind'] == 'readiness'])
    missing_usage = sum(row['usage_status'] == 'missing' for row in rows)
    usage_known = bool(rows) and not missing_usage
    return {
        'schema_version': 1,
        'experiment_id': next(iter(experiments), None),
        'layers': layers,
        'diagnostic_levels': {
            f'm{index}': summarize(
                [row for row in rows if row['kind'] == 'diagnostic'
                 and row['case_id'].endswith(f'-m{index}')], 89)
            for index in range(1, 5)
        },
        'calibration': {'status': 'not_in_record_contract', 'scored': None},
        'usage': {
            'input_tokens': sum(row['usage']['input_tokens'] for row in rows) if usage_known else None,
            'output_tokens': sum(row['usage']['output_tokens'] for row in rows) if usage_known else None,
            'missing_records': missing_usage,
            'reason': None if usage_known else ('missing usage records' if rows else 'no records'),
        },
        'cost': {'value': None, 'reason': 'no frozen price evidence'},
    }


def main(argv=None, *, emit=print):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest', required=True, type=Path)
    parser.add_argument('--results', required=True, type=Path, help='contract JSONL snapshot')
    parser.add_argument('--audit-only', action='store_true')
    parser.add_argument('--require-parents', action='store_true')
    parser.add_argument('--require-children', action='store_true')
    args = parser.parse_args(argv)
    try:
        manifest_bytes = args.manifest.read_bytes()
        results_bytes = args.results.read_bytes()
        report = build_report(json.loads(manifest_bytes),
                              [json.loads(line) for line in results_bytes.splitlines() if line.strip()])
        report['sources'] = {
            'manifest_sha256': hashlib.sha256(manifest_bytes).hexdigest(),
            'results_sha256': hashlib.sha256(results_bytes).hexdigest(),
        }
    except (OSError, ValueError, KeyError, TypeError):
        # Input data and schema errors may contain secrets: never echo their payload.
        emit(json.dumps({'error': 'invalid report evidence'}, sort_keys=True))
        return 2
    required = {'official': args.require_parents or not args.audit_only,
                'diagnostic': args.require_children or not args.audit_only}
    report['accepted'] = all(
        not enforce or report['layers'][kind]['scored'] == report['layers'][kind]['expected']
        for kind, enforce in required.items())
    emit(json.dumps(report, sort_keys=True, indent=2, allow_nan=False))
    return 0 if report['accepted'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
