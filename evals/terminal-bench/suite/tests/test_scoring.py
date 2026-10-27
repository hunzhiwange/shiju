"""Record the actual transport facts separately from scorer rewards."""
from datetime import datetime, timedelta, timezone
import unittest

from suite.scoring import experiment_record


class RecordTests(unittest.TestCase):
    def record(self, transport, reward=0):
        manifest = {'experiment_id': 'offline-record', 'adapter': {'sha256': 'a' * 64},
                    'runtime': {'binarySha256': 'b' * 64}}
        task = {'taskId': 'parent-m1', 'kind': 'diagnostic', 'files': {}}
        reservation = {'attempt_id': 'one', 'started_at':
                       (datetime.now(timezone.utc) - timedelta(seconds=60)).isoformat()}
        return experiment_record(manifest, task, reservation,
            {'agent_result': {'metadata': {'vibewindow': transport}},
             'verifier_result': {'rewards': {'reward': reward}}}, 'c' * 64)

    def normal(self):
        return {'status': 'completed', 'exit_code': 0,
                'terminal': {'type': 'chat.done'}, 'finish_reason': 'stop',
                'stream_terminated': True, 'error_seen': False,
                'usage': {'input_tokens': 100, 'output_tokens': 5}}

    def test_completed_transport_retains_official_zero_as_capability_failure(self):
        row = self.record(self.normal())
        self.assertEqual(row['status'], 'completed')
        self.assertEqual(row['reward'], 0)
        self.assertEqual(row['failure_class'], 'capability')

    def test_terminal_and_reward_cannot_hide_missing_stream_or_prior_error(self):
        for change in [{'stream_terminated': False}, {'error_seen': True}, {'finish_reason': 'length'}]:
            row = self.record({**self.normal(), **change}, 1)
            self.assertEqual(row['score_status'], 'unscored')
            self.assertIsNone(row['reward'])
            self.assertEqual(row['failure_class'], 'infrastructure')
