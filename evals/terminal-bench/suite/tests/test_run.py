"""Crash boundaries must never turn resume into a second model request."""
import json
from pathlib import Path
import tempfile
import unittest

from suite.run import atomic_json, reserve, reconcile, summarize, recovery_budget
from suite.state import digest


class ResumeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def test_crash_before_worker_claim_never_reissues_reserved_attempt(self):
        first = reserve(self.root, 'parent', 'a' * 64)
        self.assertEqual(reconcile(first)['state'], 'unknown')
        with self.assertRaises(FileExistsError):
            reserve(self.root, 'parent', 'a' * 64)

    def test_stale_pid_is_unknown_not_failure_or_permission_to_retry(self):
        attempt = reserve(self.root, 'parent', 'a' * 64)
        atomic_json(attempt / 'owner.json', {'pid': 99999999, 'start': 'stale'})
        self.assertEqual(reconcile(attempt)['state'], 'unknown')
        self.assertIsNone(reconcile(attempt)['reward'])

    def test_crash_after_scoring_recovers_exact_terminal_bytes(self):
        attempt = reserve(self.root, 'parent', 'a' * 64)
        terminal = {'state': 'finished', 'reward': 0, 'score_status': 'scored', **{k:json.loads((attempt/'reservation.json').read_text())[k] for k in ('attempt_id','case_id')}}
        atomic_json(attempt / 'terminal.json', terminal)
        before = (attempt / 'terminal.json').read_bytes()
        for _ in range(3):
            self.assertEqual(reconcile(attempt), terminal)
        self.assertEqual((attempt / 'terminal.json').read_bytes(), before)

    def test_missing_reward_is_unscored_even_if_transport_completed(self):
        result = summarize({'agent_result': {'metadata': {'vibewindow': {
            'status': 'completed'}}}, 'verifier_result': None})
        self.assertEqual(result['score_status'], 'unscored')
        self.assertIsNone(result['reward'])

    def test_api_error_nonretryable_and_dependency_failure_unscored(self):
        for kind in ('APIError', 'DependencyDownloadError'):
            result = summarize({'exception_info': {'exception_type': kind},
                                'verifier_result': {'rewards': {'reward': 1}}})
            self.assertEqual(result['failure_class'], 'infrastructure')
            self.assertIsNone(result['reward'])

    def test_official_reward_and_normal_terminal_are_independent(self):
        result = summarize({'verifier_result': {'rewards': {'reward': 0}},
                            'agent_result': {'metadata': {'vibewindow': {
                                'status': 'completed', 'terminal_event': 'chat.done'}}}})
        self.assertEqual(result['transport_status'], 'completed')
        self.assertEqual(result['reward'], 0)
        self.assertEqual(result['failure_class'], 'capability')

    def test_recovery_rejects_changed_artifact_or_uncalibrated_image(self):
        receipt = {'mode': 'artifact', 'artifact_sha256': 'a',
                   'calibrated': True, 'verifier_limit': 120, 'elapsed': 30}
        self.assertEqual(recovery_budget(receipt, 'a'), 90)
        with self.assertRaises(ValueError):
            recovery_budget(receipt, 'b')
        receipt['calibrated'] = False
        with self.assertRaises(ValueError):
            recovery_budget(receipt, 'a')

    def test_service_recovery_requires_original_environment_and_remaining_time(self):
        receipt = {'mode': 'service', 'original_environment': False,
                   'verifier_limit': 120, 'elapsed': 30}
        with self.assertRaises(ValueError):
            recovery_budget(receipt, None)
        receipt.update(original_environment=True, elapsed=120)
        with self.assertRaises(ValueError):
            recovery_budget(receipt, None)


if __name__ == '__main__':
    unittest.main()

# The fake worker uses a real OS process, a held lease and a countable loopback
# HTTP service. Crash exits cannot run Python finally blocks.
import asyncio
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import os
import subprocess
import sys
import threading
import time
from unittest.mock import patch

from suite.run import schedule, harbor_trial, tree, scrub
from suite.preparation import harbor_identity

FAKE_WORKER = r'''
import json,os,sys,time,urllib.request
from pathlib import Path
from suite.run import locked,atomic_json,process_identity
attempt=Path(sys.argv[1]); boundary=sys.argv[2]; url=sys.argv[3]
with locked(attempt/'worker.lock'):
    atomic_json(attempt/'owner.json',{'pid':os.getpid(),'start':process_identity(os.getpid())})
    if boundary=='before_request': os._exit(7)
    urllib.request.urlopen(url+'/'+attempt.name,data=b'{}').read()
    if boundary in ('after_request','before_score'): os._exit(7)
    time.sleep(.4)
    atomic_json(attempt/'raw-score.json',{'reward':1})
    if boundary=='after_score': os._exit(7)
    atomic_json(attempt/'terminal.json',{'state':'finished','score_status':'scored','reward':1,**{k:json.loads((attempt/'reservation.json').read_text())[k] for k in ('attempt_id','case_id')}})
'''


class ProcessRecoveryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.counts = {}
        counts = self.counts
        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                counts[self.path] = counts.get(self.path, 0) + 1
                self.send_response(200)
                self.end_headers()
                self.wfile.write(b'{}')
            def log_message(self, *args):
                pass
        self.server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.addCleanup(self.server.server_close)
        self.addCleanup(self.server.shutdown)
        self.url = f'http://127.0.0.1:{self.server.server_port}'
        self.manifest = self.root / 'manifest.json'
        atomic_json(self.manifest, {'experiment_id': 'local-only', 'tasks': [
            {'taskId': name, 'parent_id': 'parent'}
            for name in ['parent', 'parent-m1', 'parent-m2', 'parent-m3', 'parent-m4']]})
        self.processes = []
        self.addCleanup(self.stop_workers)

    def stop_workers(self):
        for process in self.processes:
            if process.poll() is None:
                process.terminate()
            process.wait(timeout=10)

    def launch(self, boundary):
        def start(attempt):
            process = subprocess.Popen([sys.executable, '-c', FAKE_WORKER, str(attempt), boundary, self.url],
                                       start_new_session=True)
            self.processes.append(process)
        return start

    def run_schedule(self, output, resume=False, boundary='finished'):
        return schedule(self.manifest, 'parent', output, resume,
                        launch=self.launch(boundary), validate=lambda path: json.loads(path.read_text()))

    def test_real_process_crashes_before_after_request_and_score_never_resend(self):
        for boundary in ('before_request', 'after_request', 'before_score', 'after_score'):
            with self.subTest(boundary=boundary):
                output = self.root / boundary
                attempt = reserve(output, 'parent', digest(self.manifest))
                self.launch(boundary)(attempt)
                self.processes[-1].wait(timeout=10)
                before = dict(self.counts)
                atomic_json(output / 'experiment.json', {
                    'manifest_sha256': __import__('suite.run', fromlist=['digest']).digest(self.manifest),
                    'experiment_id': 'local-only'})
                # Speed up only the launch-grace clock; real workers already exited.
                with patch('suite.run.time.monotonic', side_effect=[0, 6]):
                    with self.assertRaisesRegex(RuntimeError, 'unknown'):
                        self.run_schedule(output, True)
                self.assertEqual(self.counts, before)
                self.assertFalse((output / 'attempts/parent-m1').exists())
                self.assertEqual(reconcile(attempt)['state'], 'unknown')

    def test_repeat_resume_does_not_change_history_or_call_counts(self):
        output = self.root / 'repeat'
        self.assertEqual(len(self.run_schedule(output)), 5)
        evidence = {p: p.read_bytes() for p in output.rglob('terminal.json')}
        for _ in range(2):
            self.assertEqual(len(self.run_schedule(output, True)), 5)
        self.assertEqual(self.counts, {'/' + name: 1 for name in
                                     ['parent', 'parent-m1', 'parent-m2', 'parent-m3', 'parent-m4']})
        self.assertTrue(all(p.read_bytes() == raw for p, raw in evidence.items()))

    def test_resume_attaches_live_owned_worker(self):
        output = self.root / 'live'
        attempt = reserve(output, 'parent', digest(self.manifest))
        atomic_json(output / 'experiment.json', {
            'manifest_sha256': __import__('suite.run', fromlist=['digest']).digest(self.manifest),
            'experiment_id': 'local-only'})
        self.launch('finished')(attempt)
        deadline = time.monotonic() + 10
        while not (attempt / 'owner.json').exists() and time.monotonic() < deadline:
            time.sleep(.01)
        self.assertEqual(reconcile(attempt)['state'], 'running')
        self.assertEqual(len(self.run_schedule(output, True)), 5)
        self.assertEqual(self.counts['/parent'], 1)

    def test_manifest_mismatch_blocks_resume_before_launch(self):
        output = self.root / 'mismatch'
        atomic_json(output / 'experiment.json', {'experiment_id': 'other'})
        with self.assertRaisesRegex(ValueError, 'matching'):
            self.run_schedule(output, True)
        self.assertFalse(self.processes)

    def test_secret_redaction_preserves_other_bytes(self):
        path = self.root / 'log'
        path.write_bytes(b'before token123 after')
        scrub(self.root, 'token123')
        self.assertEqual(path.read_bytes(), b'before [REDACTED] after')


class HarborContainerTests(unittest.TestCase):
    def test_pinned_harbor_and_real_container_oracle_nop(self):
        self.assertEqual(harbor_identity()['version'], '0.24.0')
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            task = root / 'tiny'
            for directory in ('environment', 'tests', 'solution'):
                (task / directory).mkdir(parents=True)
            image = os.environ.get('VW_DIAGNOSTIC_TEST_IMAGE',
                'vw-qwen-sqlite-verifier-deps@sha256:'
                '501ed2aec3dd40c3bc8ef631ee3fb55ccbce56b22cc7bed912e71b986b86dd25')
            (task / 'instruction.md').write_text('Write 4 to /app/result.txt.\n')
            (task / 'environment/Dockerfile').write_text(f'FROM {image}\nWORKDIR /app\n')
            (task / 'solution/solve.sh').write_text('#!/bin/sh\nprintf 4 > /app/result.txt\n')
            (task / 'tests/test.sh').write_text(
                '#!/bin/sh\nmkdir -p /logs/verifier\n'
                'if [ "$(cat /app/result.txt 2>/dev/null)" = 4 ]; then '
                'echo 1; else echo 0; fi > /logs/verifier/reward.txt\n')
            (task / 'task.toml').write_text(
                'schema_version = "1.1"\n[agent]\ntimeout_sec = 10\n'
                '[verifier]\ntimeout_sec = 10\n[environment]\ncpus = 1\n'
                'memory_mb = 256\nstorage_mb = 128\n'
                f'docker_image = "{image}"\n')
            for agent, expected, mode in [('oracle', 1, 'service'), ('nop', 0, 'service'),
                                          ('oracle', 1, 'artifact'), ('nop', 0, 'artifact')]:
                with self.subTest(agent=agent, mode=mode):
                    recovery = {'mode': mode, 'paths': ['/app/result.txt'], 'image': image,
                                'calibrated': True, 'calibrate': mode == 'artifact'}
                    name = agent + '-' + mode
                    result = asyncio.run(harbor_trial(task, root / 'trials', name, agent,
                                                      recovery=recovery))
                    self.assertEqual(summarize(result)['reward'], expected, result.get('exception_info'))
                    scoring = root / 'trials' / name / 'scoring'
                    self.assertTrue((scoring / 'first.json').exists())
                    if mode == 'artifact':
                        self.assertEqual(json.loads((scoring / 'recovered.json').read_text())['rewards']['reward'], expected)


class AdmissionTests(unittest.TestCase):
    def test_unknown_attempt_blocks_other_experiments_and_duplicate_outputs(self):
        from suite import state
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(state, 'GLOBAL_LOCK', root / 'model.lock'):
                manifest = {'experiment_id': 'one', 'manifest_sha256': 'a' * 64}
                first = state.admit(root / 'out', 'parent', manifest)
                with self.assertRaisesRegex(ValueError, 'another output'):
                    state.admit(root / 'duplicate', 'parent', manifest)
                with self.assertRaisesRegex(RuntimeError, 'still running or unknown'):
                    state.admit(root / 'other', 'parent', {**manifest, 'experiment_id': 'two'})
                terminal = {**json.loads((first / 'reservation.json').read_text()), 'state': 'finished'}
                atomic_json(first / 'terminal.json', terminal)
                second = state.admit(root / 'out', 'parent-m1', manifest)
                self.assertNotEqual(first, second)

    def test_terminal_from_another_attempt_cannot_clear_barrier(self):
        with tempfile.TemporaryDirectory() as directory:
            attempt = reserve(Path(directory), 'parent', 'a' * 64)
            atomic_json(attempt / 'terminal.json', {'attempt_id': 'wrong', 'case_id': 'parent',
                                                   'state': 'finished'})
            with self.assertRaisesRegex(ValueError, 'identity mismatch'):
                reconcile(attempt)
