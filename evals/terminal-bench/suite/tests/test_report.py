"""Deterministic synthetic reporting; no model, Docker or hidden task material."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

from suite.tests.test_contract import record
from suite.report import build_report, main

CATALOG = json.loads(Path(__file__).parents[1].joinpath('catalog.json').read_text())
PARENT = CATALOG['tasks'][0]['id']


def result(kind='official', case_id=PARENT, **changes):
    value = record()
    value.update(kind=kind, case_id=case_id, attempt_id=kind + ':' + case_id)
    value.update(changes)
    return value


class ReportTests(unittest.TestCase):
    def test_empty_snapshot_has_separate_fixed_denominators(self):
        report = build_report(CATALOG, [])
        for kind, denominator in [('official', 89), ('diagnostic', 356)]:
            layer = report['layers'][kind]
            self.assertEqual(layer['expected'], denominator)
            self.assertEqual(layer['missing'], denominator)
            self.assertEqual(layer['scored'], 0)
            self.assertIsNone(layer['mean_reward'])
        self.assertEqual(report['cost'], {'value': None, 'reason': 'no frozen price evidence'})
        self.assertIsNone(report['usage']['input_tokens'])
        self.assertTrue(report['usage']['reason'])
        self.assertEqual(report, build_report(CATALOG, []))

    def test_stop_zero_is_capability_failure_and_not_infrastructure(self):
        layer = build_report(CATALOG, [result()])['layers']['official']
        self.assertEqual((layer['scored'], layer['zero_reward'], layer['normal_terminal']), (1, 1, 1))
        self.assertEqual(layer['infrastructure_failed'], 0)
        self.assertEqual(layer['mean_reward'], 0)
        self.assertEqual(layer['missing'], 88)

    def test_diagnostic_levels_and_fractional_reward_remain_independent(self):
        rows = [result('diagnostic', PARENT + '-m1', reward=0.5),
                result('diagnostic', PARENT + '-m2', reward=1, failure_class='none')]
        report = build_report(CATALOG, rows)
        self.assertEqual(report['layers']['diagnostic']['mean_reward'], 0.75)
        self.assertEqual(report['layers']['diagnostic']['missing'], 354)
        self.assertEqual(report['diagnostic_levels']['m1']['mean_reward'], 0.5)
        self.assertEqual(report['diagnostic_levels']['m2']['passed'], 1)
        self.assertEqual(report['diagnostic_levels']['m3']['missing'], 89)

    def test_duplicate_attempt_or_case_cannot_inflate_coverage(self):
        first = result()
        for second in [first, first | {'attempt_id': 'another'},
                       result('diagnostic', PARENT + '-m1', attempt_id=first['attempt_id'])]:
            with self.assertRaises(ValueError):
                build_report(CATALOG, [first, second])

    def test_missing_reward_rejected_then_recovered_score_counted_once(self):
        lost = result(reward=None)
        with self.assertRaises(ValueError):
            build_report(CATALOG, [lost])
        lost.update(status='failed', score_status='unscored', failure_class='infrastructure')
        layer = build_report(CATALOG, [lost])['layers']['official']
        self.assertEqual((layer['scored'], layer['unscored'], layer['infrastructure_failed']), (0, 1, 1))
        recovered = result(reward=1, failure_class='none')
        self.assertEqual(build_report(CATALOG, [recovered])['layers']['official']['passed'], 1)
        with self.assertRaises(ValueError):
            build_report(CATALOG, [lost, recovered])

    def test_missing_usage_does_not_become_zero(self):
        report = build_report(CATALOG, [result()])
        self.assertIsNone(report['usage']['output_tokens'])
        self.assertEqual(report['usage']['missing_records'], 1)
        known = result(usage_status='reported', usage={'input_tokens': 12, 'output_tokens': 3})
        report = build_report(CATALOG, [known])
        self.assertEqual(report['usage']['input_tokens'], 12)
        self.assertIsNone(report['cost']['value'])
        report = build_report(CATALOG, [known, result('diagnostic', PARENT + '-m1')])
        self.assertIsNone(report['usage']['input_tokens'])

    def test_length_with_partial_artifact_is_unscored_and_truncation_visible(self):
        truncated = result(status='failed', score_status='unscored', reward=None,
                           failure_class='infrastructure', finish_reason='length', output_truncated=True)
        layer = build_report(CATALOG, [truncated])['layers']['official']
        self.assertEqual((layer['truncated'], layer['unscored'], layer['zero_reward']), (1, 1, 0))
        self.assertEqual(layer['finish_reasons'], {'length': 1})

    def test_unknown_cases_mixed_experiments_and_invalid_manifest_rejected(self):
        for rows in [[result(case_id='old-five')], [result('diagnostic', PARENT + '-m5')],
                     [result(), result('diagnostic', PARENT + '-m1', experiment_id='old')]]:
            with self.assertRaises(ValueError):
                build_report(CATALOG, rows)
        manifest = copy.deepcopy(CATALOG)
        manifest['tasks'][1]['id'] = PARENT
        with self.assertRaises(ValueError):
            build_report(manifest, [])

    def test_each_of_445_cases_has_independent_missing_coverage(self):
        for task in CATALOG['tasks']:
            for kind, name, expected in [('official', task['id'], 89)] + [
                    ('diagnostic', task['id'] + '-m' + str(i), 356) for i in range(1, 5)]:
                with self.subTest(kind=kind, case_id=name):
                    layer = build_report(CATALOG, [result(kind, name)])['layers'][kind]
                    self.assertEqual(layer['observed'], 1)
                    self.assertEqual(layer['missing'], expected - 1)
                    self.assertEqual(layer['coverage'], 1 / expected)

    def test_readiness_is_separate_and_output_contains_no_record_payload(self):
        value = result('readiness', 'synthetic-probe')
        report = build_report(CATALOG, [value])
        self.assertEqual(report['layers']['readiness']['scored'], 1)
        self.assertEqual(report['layers']['official']['missing'], 89)
        self.assertEqual(report['calibration']['status'], 'not_in_record_contract')
        self.assertNotIn(value['hashes']['fixture'], json.dumps(report))

    def test_full_coverage_and_each_single_missing_case_fail_strict_cli(self):
        rows = []
        for task in CATALOG['tasks']:
            rows.append(result(case_id=task['id']))
            rows.extend(result('diagnostic', task['id'] + '-m' + str(i)) for i in range(1, 5))
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / 'manifest.json'
            results = Path(directory) / 'results.jsonl'
            manifest.write_text(json.dumps(CATALOG))
            args = ['--manifest', str(manifest), '--results', str(results)]
            def run(values, extra=()):
                results.write_text(''.join(json.dumps(row) + '\n' for row in values))
                return main(args + list(extra), emit=lambda _: None)
            self.assertEqual(run(rows), 0)
            for index in (0, 1, len(rows) - 1):
                missing = rows[:index] + rows[index + 1:]
                self.assertEqual(run(missing), 1)
                self.assertEqual(run(missing, ['--audit-only']), 0)
                flag = '--require-parents' if rows[index]['kind'] == 'official' else '--require-children'
                self.assertEqual(run(missing, ['--audit-only', flag]), 1)
            self.assertEqual(run([result(status='failed', score_status='unscored', reward=None,
                                         failure_class='infrastructure')]), 1)
            results.write_text('{broken\n')
            self.assertEqual(main(args, emit=lambda _: None), 2)
