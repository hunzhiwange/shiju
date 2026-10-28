"""Run/resume one parent followed by m1..m4 using a frozen suite manifest.

Runtime JSON for preflight/freeze supplies absolute historical_root, source_root,
cases_root and tasks {taskId, workingDirectory, agentTimeoutSec}. Preparation
receipts live at cases_root/PARENT/preparation.json. freeze requires 89 calibrated
receipts plus diagnostic validation.json. No model runs during preparation.
Credentials enter run/resume only through VW_SUITE_PROFILE (temporary 0600 JSON).
An ambiguous reserved attempt is never relaunched; use an explicitly new frozen
experiment after investigation. All parents in one experiment share --output.
"""
import argparse
import asyncio
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

from suite.state import (MODEL, GLOBAL_LOCK, load, digest, atomic_json, locked,
                         process_identity, reserve, admit, reconcile, tree)
from suite.scoring import (summarize, experiment_record, recovery_budget,
                           harbor_trial, scrub)
from suite.preparation import preflight, freeze, validate_manifest

def schedule(manifest_path, parent, output, resume=False, launch=None, validate=validate_manifest):
    manifest_path, output = Path(manifest_path).resolve(), Path(output).resolve()
    manifest = validate(manifest_path)
    tasks = [task for task in manifest['tasks'] if task['parent_id'] == parent]
    if ([t['taskId'] for t in tasks] != [parent] + [parent + '-m' + str(n) for n in range(1, 5)]):
        raise ValueError('parent must be followed by m1..m4')
    output.mkdir(parents=True, exist_ok=True)
    with locked(output / 'scheduler.lock'):
        experiment = {'manifest_sha256': digest(manifest_path), 'experiment_id': manifest['experiment_id']}
        if launch is None:
            from suite.agent import _validate_profile
            public = _validate_profile(os.environ.get('VW_SUITE_PROFILE'))['provider']
            experiment['provider_sha256'] = hashlib.sha256(
                json.dumps(public, sort_keys=True).encode()).hexdigest()
        if (output / 'experiment.json').exists():
            if (load(output / 'experiment.json') != experiment or
                    (not resume and (output / 'attempts' / parent).exists())):
                raise ValueError('existing experiment requires matching resume')
        else:
            atomic_json(output / 'experiment.json', experiment)
        results = []
        for task in tasks:
            attempt = output / 'attempts' / task['taskId']
            if not attempt.exists():
                attempt = (reserve(output, task['taskId'], experiment['manifest_sha256']) if launch else
                           admit(output, task['taskId'], experiment))
                if launch:
                    launch(attempt)
                else:
                    subprocess.Popen([sys.executable, '-m', 'suite.run', '_worker',
                                      '--manifest', str(manifest_path), '--output', str(attempt)],
                                     stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                     stderr=subprocess.DEVNULL, start_new_session=True)
            reservation = load(attempt / 'reservation.json')
            if (reservation['manifest_sha256'] != experiment['manifest_sha256']
                    or reservation['case_id'] != task['taskId']):
                raise ValueError('reservation does not belong to frozen experiment')
            # Worker startup can race ownership publication. Never relaunch it.
            deadline = time.monotonic() + 5
            while True:
                state = reconcile(attempt)
                if state['state'] == 'finished':
                    if (attempt / 'record.json').exists():
                        from suite.contract import append_record
                        row = load(attempt / 'record.json')
                        history = output / 'history.jsonl'
                        previous = [json.loads(line) for line in history.read_text().splitlines()] if history.exists() else []
                        from suite.contract import validate_record
                        for prior in previous:
                            validate_record(prior)
                        matching = [r for r in previous if r['attempt_id'] == row['attempt_id']]
                        if matching and matching != [row]:
                            raise ValueError('existing attempt evidence differs')
                        if not matching:
                            append_record(history, row)
                    results.append(state)
                    break
                if state['state'] == 'unknown' and time.monotonic() >= deadline:
                    raise RuntimeError('unknown attempt ownership; manual investigation required')
                time.sleep(0.1)
        atomic_json(output / (parent + '-summary.json'), results)
        return results


def worker(manifest_path, attempt):
    from suite.agent import _validate_profile
    attempt = Path(attempt).resolve()
    with locked(attempt / 'worker.lock'), locked(GLOBAL_LOCK):
        if (attempt / 'owner.json').exists() or (attempt / 'terminal.json').exists():
            raise ValueError('worker already claimed; paid retry forbidden')
        atomic_json(attempt / 'owner.json', {'pid': os.getpid(), 'start': process_identity(os.getpid())})
        reservation = load(attempt / 'reservation.json')
        secret = None
        dispatched = False
        result = {}
        manifest = task = profile_sha = None
        try:
            manifest = validate_manifest(manifest_path)
            if reservation['manifest_sha256'] != digest(manifest_path):
                raise ValueError('reservation manifest drift')
            task = next(t for t in manifest['tasks'] if t['taskId'] == reservation['case_id'])
            profile_path = os.environ.get('VW_SUITE_PROFILE')
            profile = _validate_profile(profile_path)
            secret = profile['auth']['key']
            public_sha = hashlib.sha256(json.dumps(profile['provider'], sort_keys=True).encode()).hexdigest()
            if load(attempt.parents[1] / 'experiment.json')['provider_sha256'] != public_sha:
                raise ValueError('provider identity changed since experiment admission')
            profile_sha = digest(profile_path)
            kwargs = {'task_id': task['taskId'], 'runtime_manifest': str(manifest_path),
                      'runtime_dir': str(Path(manifest['runtime_root']) / 'runtime'),
                      'profile_path': profile_path}

            async def hook(event):
                # Container IDs are discovered by Harbor's unique compose session;
                # never attach to a container based on task name or stale PID alone.
                project = re.sub('[^a-z0-9_-]', '', (reservation['attempt_id'] + '__env').lower())
                containers = subprocess.run(['docker', 'ps', '-aq', '--filter',
                                            'label=com.docker.compose.project=' + project],
                                           capture_output=True, text=True, timeout=30, check=True)
                phase = {'phase': event.event.value,
                            'trial_id': str(event.trial_id), 'session_id': project,
                            'container_ids': containers.stdout.split(),
                            'timestamp': event.timestamp.isoformat()}
                with (attempt / 'phases.jsonl').open('a') as history:
                    history.write(json.dumps(phase, sort_keys=True) + '\n')
                    history.flush()
                    os.fsync(history.fileno())
                atomic_json(attempt / 'phase.json', phase)

            dispatched = True
            result = asyncio.run(harbor_trial(task['path'], attempt / 'harbor',
                                 reservation['attempt_id'], 'suite.agent:SuiteAgent', kwargs, hook,
                                 task.get('recovery'),
                                 **({'startup_override': task['startup_override']}
                                    if 'startup_override' in task else {}),
                                 **({'environment_variant': task['environment_variant']}
                                    if 'environment_variant' in task else {})))
            atomic_json(attempt / 'raw-result.json', result)
            summary = summarize(result)
            transport = (((result.get('agent_result') or {}).get('metadata') or {}).get('vibewindow') or {})
            if not transport or any(transport.get(key) != 0 for key in
                                    ('cleanup_exit_code', 'profile_cleanup_exit_code', 'stop_exit_code')):
                summary['state'] = 'unknown'  # An orphaned request blocks subsequent paid work.
        except Exception as error:
            # Exception text can contain a URL/token. Record only its class.
            summary = {'state': 'unknown' if dispatched else 'finished', 'transport_status': 'unknown',
                       'score_status': 'unscored', 'reward': None,
                       'failure_class': 'infrastructure', 'error_type': type(error).__name__}
        finally:
            scrub(attempt, secret)
        if manifest and task and profile_sha:
            record = experiment_record(manifest, task, reservation, result, profile_sha)
            atomic_json(attempt / 'record.json', record)
        started = int(datetime.fromisoformat(reservation['started_at']).timestamp() * 1000)
        stopped = int(time.time() * 1000)
        atomic_json(attempt / 'ctrf.json', {'results': {'tool': {'name': 'vibewindow-suite'},
                    'summary': {'tests': 1, 'passed': int(summary['reward'] == 1),
                                'failed': int(summary['reward'] is not None and summary['reward'] < 1),
                                'skipped': int(summary['reward'] is None), 'pending': 0, 'other': 0,
                                'start': started, 'stop': stopped},
                    'tests': [{'name': reservation['case_id'], 'duration': stopped - started,
                               'status': 'skipped' if summary['reward'] is None else
                                         'passed' if summary['reward'] == 1 else 'failed'}]}})
        summary.update(attempt_id=reservation['attempt_id'], case_id=reservation['case_id'],
                       attempt_number=1, evidence=tree(attempt / 'harbor'))
        atomic_json(attempt / 'terminal.json', summary)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    for command in ('preflight', 'freeze', 'run', 'resume', '_worker'):
        sub = commands.add_parser(command)
        sub.add_argument('--output', type=Path, required=True)
        if command in ('preflight', 'freeze'):
            sub.add_argument('--catalog', type=Path, required=True)
            sub.add_argument('--runtime', type=Path, required=True)
            sub.add_argument('--parent-id' if command == 'preflight' else '--cases-root', required=True)
        else:
            sub.add_argument('--manifest', type=Path, required=True)
            if command != '_worker':
                sub.add_argument('--parent-id', required=True)
    args = parser.parse_args()
    if args.command == 'preflight':
        preflight(args.catalog, args.runtime, args.parent_id, args.output)
    elif args.command == 'freeze':
        freeze(args.catalog, args.cases_root, args.runtime, args.output)
    elif args.command == '_worker':
        worker(args.manifest.resolve(), args.output)
    else:
        schedule(args.manifest, args.parent_id, args.output, args.command == 'resume')


if __name__ == '__main__':
    main()
