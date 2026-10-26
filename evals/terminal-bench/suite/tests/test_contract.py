"""Synthetic experiment records; never calls a model."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

from suite.contract import append_record, validate_record


def record():
    return {
        'schema_version': 1, 'experiment_id': 'synthetic', 'case_id': 'example',
        'attempt_id': 'attempt-1', 'attempt_number': 1, 'kind': 'official',
        'model': 'token-plan/qwen3.8-flash', 'max_output_tokens': 32000,
        'input_window_tokens': 200000,
        'hashes': {key: 'a' * 64 for key in ('source', 'runtime', 'fixture', 'checker', 'profile')},
        'started_at': '2026-10-27T00:00:00Z', 'ended_at': '2026-10-27T00:01:00Z',
        'validation_seconds': 2.5, 'usage_status': 'missing', 'usage': None,
        'raw_exit_code': 0, 'terminal_event': 'chat.done', 'finish_reason': 'stop',
        'stream_terminated': True, 'error_seen': False, 'output_truncated': False,
        'status': 'completed', 'score_status': 'scored', 'reward': 0,
        'failure_class': 'capability',
    }


class ContractTests(unittest.TestCase):
    def reject(self, **changes):
        value = record()
        value.update(changes)
        with self.assertRaises(ValueError):
            validate_record(value)

    def test_zero_score_and_normal_stop_are_independent(self):
        validate_record(record())
        value = record()
        value.update(reward=1, failure_class='none')
        validate_record(value)

    def test_unknown_reason_is_rejected(self):
        self.reject(finish_reason='mystery')

    def test_truncation_cannot_be_completed(self):
        self.reject(output_truncated=True)
        self.reject(finish_reason='length')

    def test_official_score_is_required_for_completed(self):
        self.reject(score_status='unscored', reward=None)

    def test_model_and_budget_are_frozen(self):
        for change in ({'model': 'other'}, {'max_output_tokens': 32001},
                       {'input_window_tokens': 200001}, {'attempt_number': 2}):
            self.reject(**change)

    def test_errors_cannot_forge_success(self):
        for change in ({'raw_exit_code': 1}, {'terminal_event': 'chat.error'},
                       {'error_seen': True}, {'stream_terminated': False}):
            self.reject(**change)

    def test_infrastructure_failure_is_unscored(self):
        value = record()
        value.update(status='failed', score_status='unscored', reward=None,
                     failure_class='infrastructure', raw_exit_code=1,
                     terminal_event=None, finish_reason=None, stream_terminated=False)
        validate_record(value)
        value['reward'] = 0
        with self.assertRaises(ValueError):
            validate_record(value)

    def test_usage_missing_is_explicit(self):
        self.reject(usage_status='reported')
        value = record()
        value.update(usage_status='reported', usage={'input_tokens': 3, 'output_tokens': 2})
        validate_record(value)
        self.reject(usage={'input_tokens': 3, 'output_tokens': 2})

    def test_schema_rejects_unknown_fields_and_invalid_types(self):
        self.reject(secret='never')
        self.reject(raw_exit_code=True)
        self.reject(validation_seconds=-1)
        self.reject(hashes={})
        self.reject(started_at='yesterday')
        self.reject(ended_at='2026-10-26T00:00:00Z')
        self.reject(reward=float('nan'))

    def test_all_record_kinds_are_supported(self):
        for kind in ('official', 'diagnostic', 'readiness'):
            value = record()
            value['kind'] = kind
            validate_record(value)
        self.reject(kind='calibration')

    def test_duplicate_first_attempt_and_identity_are_rejected_without_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'history.jsonl'
            first = record()
            append_record(path, first)
            before = path.read_bytes()
            second = copy.deepcopy(first)
            second['attempt_id'] = 'attempt-2'
            for value in (first, second):
                with self.assertRaises(ValueError):
                    append_record(path, value)
                self.assertEqual(path.read_bytes(), before)
            second['case_id'] = 'another'
            with self.assertRaises(ValueError):
                append_record(path, second | {'attempt_id': first['attempt_id']})
            append_record(path, second)
            self.assertTrue(path.read_bytes().startswith(before))
            self.assertEqual(len(path.read_text().splitlines()), 2)

    def test_invalid_history_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'history.jsonl'
            path.write_text('{}\n')
            with self.assertRaises(ValueError):
                append_record(path, record())
            self.assertEqual(path.read_text(), '{}\n')
