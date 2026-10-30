"""Calibrate only the approved compatibility identity, without model calls.

Writes a separate compatibility receipt. It cannot create preparation.json or
mark the original official parent ready. Children use standard Harbor scoring.
"""
import argparse
import asyncio
import fcntl
import json
from pathlib import Path
import re
import tempfile
import time

from materialize import PARENT, VARIANT, materialize
from suite import diagnostics
from suite.docker import LOCK
from suite.preparation import CATALOG, inputs
from suite.scoring import harbor_trial, summarize
from suite.state import atomic_json, digest, load, tree


def wait_for_builder():
    # Wait for existing work without deleting locks or holding a nested lock.
    with LOCK.open('a') as stream:
        fcntl.flock(stream, fcntl.LOCK_EX)
        fcntl.flock(stream, fcntl.LOCK_UN)


def calibrate_children(output):
    case = Path(__file__).resolve().parent.parent / 'case.json'
    diagnostics.materialize(case, output / 'children')
    rows = []
    for task in sorted((output / 'children').iterdir()):
        for agent, expected in [('oracle', 1), ('nop', 0)]:
            for attempt in range(1, 61):
                wait_for_builder()
                result = asyncio.run(harbor_trial(task, output / 'child-calibration',
                                    task.name + '-' + agent + '-' + str(attempt), agent))
                error = result.get('exception_info') or {}
                if error.get('exception_type') == 'BlockingIOError':
                    time.sleep(5)
                    continue
                break
            summary = summarize(result)
            rows.append({'case_id': task.name, 'agent': agent, 'result': result,
                         'summary': summary, 'task_files': tree(task)})
            atomic_json(output / 'children-receipt.json', rows)
            print(task.name, agent, summary, flush=True)
            if summary['score_status'] != 'scored' or summary['reward'] != expected:
                raise ValueError('child calibration failed')
    return output / 'children-receipt.json'


def run(runtime_path, output, probe_path, children_path):
    runtime, source_receipt = inputs(CATALOG, runtime_path, PARENT)
    probe = load(probe_path)
    if (probe['exit_code'] != 0 or probe['cleanup_exit_code'] != 0
            or 'COMPAT_LIFECYCLE_PROBE_PASSED' not in probe['stdout']):
        raise ValueError('successful lifecycle/error probe required')
    output.mkdir(exist_ok=False, parents=True)
    children_path = children_path or calibrate_children(output)
    children = load(children_path)
    wanted = [(PARENT + '-m' + str(n), agent, reward)
              for n in range(1, 5) for agent, reward in [('oracle', 1), ('nop', 0)]]
    if [(r['case_id'], r['agent'], r['summary']['reward']) for r in children] != wanted:
        raise ValueError('eight successful child calibrations required')
    if any(summarize(r['result']) != r['summary'] or r['summary']['score_status'] != 'scored'
           for r in children):
        raise ValueError('invalid child calibration')
    child_evidence = []
    with tempfile.TemporaryDirectory() as directory:
        generated = Path(directory) / 'tasks'
        diagnostics.materialize(Path(__file__).resolve().parent.parent / 'case.json', generated)
        for row in children:
            config = row['result']['config']
            task_path = Path(config['task']['path'])
            trial_path = Path(config['trials_dir']) / config['trial_name']
            first = load(trial_path / 'scoring' / 'first.json')
            if (tree(generated / row['case_id']) != row['task_files']
                    or tree(task_path) != row['task_files']
                    or first.get('rewards', {}).get('reward') != row['summary']['reward']):
                raise ValueError('child task/scorer evidence drift')
            child_evidence.append({'case_id': row['case_id'], 'agent': row['agent'],
                                   'trial_path': str(trial_path),
                                   'scoring_files': tree(trial_path / 'scoring'),
                                   'verifier_files': tree(trial_path / 'verifier')})
    task = output / VARIANT
    identity = materialize(Path(runtime['source_root']) / 'tasks' / PARENT, task)
    if (probe.get('identity') != identity
            or probe.get('probe_sha256') != digest(Path(__file__).with_name('probe.py'))):
        raise ValueError('lifecycle probe belongs to another helper or probe implementation')
    recovery = next(t['recovery'] for t in runtime['tasks'] if t['taskId'] == PARENT)
    receipt = {'identity': identity, 'kind': 'compatibility_variant',
               'source_receipt': source_receipt,
               'compat_files': {name: digest(Path(__file__).resolve().parent / name)
                                for name in ('calibrate.py', 'materialize.py', 'probe.py', 'run_probe.py',
                                             'cleanup.patch', 'infrastructure.patch', 'page-alarm.patch')},
               'counts_as_original_official': False, 'model_calls': 0,
               'runtime_sha256': digest(runtime_path), 'recovery': recovery,
               'probe': {'path': str(probe_path), 'sha256': digest(probe_path)},
               'children': {'path': str(children_path), 'sha256': digest(children_path)},
               'child_evidence': child_evidence,
               'calibration': [], 'status': 'incomplete'}
    for agent, expected in [('oracle', 1), ('nop', 0)]:
        for attempt in range(1, 61):
            wait_for_builder()
            result = asyncio.run(harbor_trial(task, output / 'calibration',
                                VARIANT + '-' + agent + '-' + str(attempt), agent, recovery=recovery))
            summary = summarize(result)
            error = result.get('exception_info') or {}
            # Infrastructure admission retries occur before any agent/scorer;
            # preserve every failed trial, never retry a completed calibration.
            if error.get('exception_type') in ('BlockingIOError', 'ValueError') and (
                    'Resource temporarily unavailable' in json.dumps(error)
                    or 'suite Docker capacity exceeds' in json.dumps(error)):
                print('Waiting for shared Docker admission', flush=True)
                time.sleep(5)
                continue
            break
        # Harbor logs are located by the exact trial name, not inferred output.
        trial = output / 'calibration' / (VARIANT + '-' + agent + '-' + str(attempt))
        logs = (trial / 'verifier' / 'test-stdout.txt').read_text()
        infra = ('Failed to load URL:', 'Error checking for alert:', 'Browser execution timed out',
                 'Failed to create driver or process file:', 'Error closing driver:')
        harbor_summary = summary.copy()
        if any(marker in logs for marker in infra):
            summary = {**summary, 'score_status': 'unscored', 'reward': None,
                       'failure_class': 'infrastructure'}
        valid = summary['score_status'] == 'scored' and summary['reward'] == expected
        if agent == 'oracle':
            batch_ids = [int(n) for n in re.findall(r'^RUN BATCH (\d+) ', logs, re.MULTILINE)]
            batches = len(batch_ids)
            vectors = re.findall(r'Filter successfully blocked all (\d+) XSS attack vectors', logs)
            total = re.findall(r'^Total batches to test: (\d+)$', logs, re.MULTILINE)
            if len(vectors) == len(total) == 1:
                valid = valid and batch_ids == list(range(int(total[0])))
                valid = valid and batches == (int(vectors[0]) + 15) // 16
            else:
                valid = False
            valid = valid and batches > 0 and logs.count('  No alert detected') == batches
            valid = valid and logs.count('  Driver closed') == batches and '2 passed' in logs
            valid = valid and 'preserved all 12 clean HTML files' in logs
            valid = valid and not any(marker in logs for marker in infra)
        else:
            batches = 0
            valid = valid and '2 failed' in logs and 'filter.py does not exist' in logs
        row = {'agent': agent, 'summary': summary, 'harbor_summary': harbor_summary,
               'result': result, 'browser_batches': batches,
               'scoring_files': tree(trial / 'scoring'), 'trial_path': str(trial),
               'verifier_files': tree(trial / 'verifier'), 'valid': valid}
        receipt['calibration'].append(row)
        atomic_json(output / 'compatibility-preparation.json', receipt)
        print(agent, summary, 'batches', batches, 'valid', valid, flush=True)
        if not valid:
            raise ValueError('compatibility calibration failed; original remains blocked')
    receipt['status'] = 'compatibility_calibrated'
    atomic_json(output / 'compatibility-preparation.json', receipt)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ('runtime', 'output', 'probe-receipt'):
        parser.add_argument('--' + flag, type=Path, required=True)
    parser.add_argument('--children-receipt', type=Path)
    args = parser.parse_args()
    run(args.runtime.resolve(), args.output.resolve(), args.probe_receipt.resolve(),
        args.children_receipt.resolve() if args.children_receipt else None)
