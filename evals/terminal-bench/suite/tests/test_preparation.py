"""Freeze the complete synthetic coverage; real tasks/scoring are not replaced."""
from contextlib import ExitStack
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from suite import preparation as p
from suite.state import MODEL, atomic_json, digest, tree
from suite.scoring import summarize


class PreparationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.catalog = self.root / 'catalog.json'
        self.parents = [f'case-{i:02d}' for i in range(89)]
        atomic_json(self.catalog, {'schema_version': 1, 'task_count': 89,
                                  'tasks': [{'id': name} for name in self.parents]})
        self.cases = self.root / 'cases'
        self.identity = {'generation': 2, 'binary_sha256': 'a' * 64,
                         'sidecars': {'libsqlite3.so': 'b' * 64, 'vw-budget-gate': 'c' * 64}}
        self.runtime = {'historical_root': str(self.root / 'runtime'),
                        'source_root': str(self.root / 'source'), 'cases_root': str(self.cases),
                        'tasks': [{'taskId': name, 'workingDirectory': '/app', 'agentTimeoutSec': 900}
                                  for name in self.parents]}
        self.current = {}
        self.stack = ExitStack()
        self.addCleanup(self.stack.close)
        patches = [patch.object(p, 'CATALOG', self.catalog),
                   patch.object(p, 'checked_runtime', return_value=(self.runtime, self.identity)),
                   patch.object(p, 'verify_runtime', return_value=self.identity),
                   patch.object(p, 'adapter_identity', return_value={'sha256': 'd' * 64}),
                   patch.object(p, 'execution_identity', return_value='e' * 64),
                   patch.object(p, 'harbor_identity', return_value={'sha256': 'f' * 64}),
                   patch.object(p.diagnostics, 'validate_all', return_value=356),
                   patch.object(p.diagnostics, 'materialize', side_effect=self.materialize)]
        for item in patches:
            self.stack.enter_context(item)
        for parent in self.parents:
            official = self.root / 'source/tasks' / parent
            official.mkdir(parents=True)
            (official / 'instruction.md').write_text('Synthetic fixture ' + parent)
            case = self.cases / parent / 'case.json'
            case.parent.mkdir(parents=True)
            case.write_text('{}')
            current = {'parent_id': parent, 'catalog_sha256': digest(self.catalog),
                       'runtime_identity': self.identity, 'adapter': {'sha256': 'd' * 64},
                       'scheduler_sha256': 'e' * 64, 'official': str(official),
                       'diagnostic': str(case), 'case_settings': p.case_settings(self.runtime, parent)}
            self.current[parent] = current
            entries = []
            for name in [parent] + [parent + '-m' + str(n) for n in range(1, 5)]:
                for agent, reward in [('oracle', 1), ('nop', 0)]:
                    scoring = self.root / 'scores' / agent
                    scoring.mkdir(parents=True, exist_ok=True)
                    atomic_json(scoring / 'first.json', {'rewards': {'reward': reward}})
                    result = {'task_name': name, 'agent_info': {'name': agent},
                              'verifier_result': {'rewards': {'reward': reward}}}
                    entries.append({'case_id': name, 'agent': agent, 'result': result,
                                    'summary': summarize(result), 'first_score': {'rewards': {'reward': reward}},
                                    'recovery_score': None, 'evidence_root': str(scoring), 'evidence_files': tree(scoring)})
            atomic_json(case.with_name('preparation.json'), {**current, 'calibration': entries})
        self.stack.enter_context(patch.object(p, 'inputs', side_effect=lambda catalog, runtime, parent, checked=None:
                                              (self.runtime, copy.deepcopy(self.current[parent]))))

    def materialize(self, case, output):
        output = Path(output)
        output.mkdir(parents=True, exist_ok=False)
        for n in range(1, 5):
            child = output / (Path(case).parent.name + '-m' + str(n))
            child.mkdir()
            (child / 'instruction.md').write_text('Independent synthetic fixture')

    def freeze(self):
        return p.freeze(self.catalog, self.cases, self.root / 'runtime.json', self.root / 'frozen')

    def test_complete_coverage_freezes_445_cases_and_rejects_task_drift(self):
        manifest = self.freeze()
        self.assertEqual(len(manifest['tasks']), 445)
        self.assertEqual(manifest['model'], MODEL)
        self.assertEqual(p.validate_manifest(self.root / 'frozen/manifest.json'), manifest)
        Path(manifest['tasks'][1]['path'], 'instruction.md').write_text('drift')
        with self.assertRaisesRegex(ValueError, 'task drift'):
            p.validate_manifest(self.root / 'frozen/manifest.json')

    def test_missing_one_preparation_receipt_cannot_freeze_partial_suite(self):
        (self.cases / self.parents[-1] / 'preparation.json').unlink()
        with self.assertRaises(OSError):
            self.freeze()
        self.assertFalse((self.root / 'frozen/manifest.json').exists())

    def test_changed_calibration_and_incomplete_parent_groups_rejected(self):
        manifest = self.freeze()
        path = self.cases / self.parents[0] / 'preparation.json'
        path.write_text('{}')
        with self.assertRaisesRegex(ValueError, 'receipt changed'):
            p.validate_manifest(self.root / 'frozen/manifest.json')
        manifest['tasks'].pop()
        atomic_json(self.root / 'frozen/manifest.json', manifest)
        with self.assertRaisesRegex(ValueError, 'complete parent groups'):
            p.validate_manifest(self.root / 'frozen/manifest.json')

    def test_earned_reward_cannot_replace_oracle_nop_calibration(self):
        path = self.cases / self.parents[0] / 'preparation.json'
        value = json.loads(path.read_text())
        value['calibration'][0]['result']['agent_info']['name'] = 'vibewindow-suite'
        atomic_json(path, value)
        with self.assertRaisesRegex(ValueError, 'calibration evidence'):
            self.freeze()

    def test_changed_execution_budget_rejected_even_before_admission(self):
        manifest = self.freeze()
        manifest['tasks'][1]['agentTimeoutSec'] = 9999
        atomic_json(self.root / 'frozen/manifest.json', manifest)
        with self.assertRaisesRegex(ValueError, 'task drift'):
            p.validate_manifest(self.root / 'frozen/manifest.json')

    def test_child_recovery_is_frozen_with_calibration_requirement(self):
        self.runtime['tasks'].append({'taskId': self.parents[0] + '-m1',
            'workingDirectory': '/app', 'agentTimeoutSec': 900,
            'recovery': {'mode': 'artifact', 'image': 'image@sha256:' + '0' * 64, 'paths': ['/app/result']}})
        settings = p.case_settings(self.runtime, self.parents[0])
        self.assertTrue(settings[self.parents[0] + '-m1']['recovery']['calibrated'])
        self.assertFalse(settings[self.parents[0] + '-m1']['recovery']['calibrate'])

    def test_preflight_runs_ten_oracle_nop_trials_without_model_agent(self):
        calls = []
        async def trial(task, directory, name, agent, recovery=None):
            calls.append((Path(task).name, agent))
            result = {'task_name': Path(task).name, 'agent_info': {'name': agent},
                      'verifier_result': {'rewards': {'reward': int(agent == 'oracle')}}}
            atomic_json(Path(directory) / name / 'scoring/first.json', result['verifier_result'])
            return result
        with patch.object(p, 'harbor_trial', side_effect=trial):
            receipt = p.preflight(self.catalog, self.root / 'runtime.json', self.parents[0],
                                  self.root / 'preflight')
        self.assertEqual(calls, [(name, agent) for name in [self.parents[0]] +
            [self.parents[0] + '-m' + str(n) for n in range(1, 5)] for agent in ('oracle', 'nop')])
        p.verify_preparation(self.cases / self.parents[0] / 'preparation.json',
                             self.current[self.parents[0]], None)
        self.assertEqual(len(receipt['calibration']), 10)

    def test_failed_preflight_retains_raw_receipt_without_admitting_models(self):
        async def trial(task, directory, name, agent, recovery=None):
            atomic_json(Path(directory) / name / 'scoring/first.json', {'error_type': 'DependencyError'})
            return {'task_name': Path(task).name, 'agent_info': {'name': agent},
                    'exception_info': {'exception_type': 'DependencyError'}}
        with patch.object(p, 'harbor_trial', side_effect=trial) as called:
            with self.assertRaisesRegex(ValueError, 'no model run permitted'):
                p.preflight(self.catalog, self.root / 'runtime.json', self.parents[0],
                            self.root / 'failed-preflight')
        self.assertEqual(called.call_count, 1)
        self.assertTrue((self.root / 'failed-preflight/preparation.json').exists())
