"""Synthetic subset admission; no containers, network, models or real manifests."""
import copy
from pathlib import Path
from unittest.mock import patch

from suite.tests import test_preparation as fixtures
from suite import preparation as p
from suite.state import atomic_json, digest


class ReadySubsetTests(fixtures.PreparationTests):
    def selection(self):
        evidence = self.root / 'blocked.txt'
        evidence.write_text('Synthetic blocked preparation evidence')
        return {'kind': 'ready_subset', 'selected': [self.parents[0]], 'excluded': [
            {'parent_id': name, 'status': 'not_ready', 'reason': 'Preparation incomplete',
             'evidence': [{'path': str(evidence), 'sha256': digest(evidence)}]}
            for name in self.parents[1:]]}

    def freeze_subset(self, selection=None):
        entry = getattr(p, 'freeze_ready_subset', None)
        self.assertTrue(callable(entry), 'missing explicit ready-subset freeze entry')
        return entry(self.catalog, self.cases, self.root / 'runtime.json',
                     self.root / 'subset', selection if selection is not None else self.selection())

    def test_subset_freezes_only_selected_complete_groups_and_preserves_exclusions(self):
        selection = self.selection()
        manifest = self.freeze_subset(selection)
        self.assertEqual(len(manifest['tasks']), 5)
        self.assertEqual(manifest['scope'], selection)
        self.assertEqual(p.validate_manifest(self.root / 'subset/manifest.json'), manifest)
        self.assertEqual(set(manifest['receipts']), {self.parents[0]})

    def test_subset_missing_selected_preparation_fails_without_silent_omission(self):
        (self.cases / self.parents[0] / 'preparation.json').unlink()
        with self.assertRaises(OSError):
            self.freeze_subset()
        self.assertFalse((self.root / 'subset/manifest.json').exists())

    def test_subset_selected_calibration_failure_is_rejected(self):
        path = self.cases / self.parents[0] / 'preparation.json'
        value = p.load(path)
        value['calibration'][0]['summary']['reward'] = 0
        atomic_json(path, value)
        with self.assertRaises(ValueError):
            self.freeze_subset()

    def test_selection_must_cover_catalog_without_duplicates_overlap_or_unknown_ids(self):
        good = self.selection()
        bad = [{}]
        none_selected = copy.deepcopy(good); none_selected['selected'] = []; bad.append(none_selected)
        mode = copy.deepcopy(good); mode['kind'] = 'full'; bad.append(mode)
        missing = copy.deepcopy(good); missing['excluded'].pop(); bad.append(missing)
        duplicate = copy.deepcopy(good); duplicate['selected'] *= 2; bad.append(duplicate)
        overlap = copy.deepcopy(good); overlap['excluded'][0]['parent_id'] = self.parents[0]; bad.append(overlap)
        unknown = copy.deepcopy(good); unknown['selected'] = ['unknown']; bad.append(unknown)
        empty = copy.deepcopy(good); empty['excluded'][0]['evidence'] = []; bad.append(empty)
        repeated = copy.deepcopy(good); repeated['excluded'].append(copy.deepcopy(repeated['excluded'][0])); bad.append(repeated)
        foreign = copy.deepcopy(good); foreign['excluded'][0]['parent_id'] = 'foreign'; bad.append(foreign)
        reason = copy.deepcopy(good); reason['excluded'][0]['reason'] = ''; bad.append(reason)
        for value in bad:
            with self.subTest(value=value.get('selected')), self.assertRaises(ValueError):
                self.freeze_subset(value)

    def test_exclusion_evidence_drift_invalidates_frozen_subset(self):
        self.freeze_subset()
        (self.root / 'blocked.txt').write_text('changed')
        with self.assertRaises(ValueError):
            p.validate_manifest(self.root / 'subset/manifest.json')

    def test_full_manifest_cannot_be_relabelled_by_removing_subset_scope(self):
        manifest = self.freeze_subset()
        manifest.pop('scope')
        atomic_json(self.root / 'subset/manifest.json', manifest)
        with self.assertRaisesRegex(ValueError, 'complete parent groups'):
            p.validate_manifest(self.root / 'subset/manifest.json')

    def test_explicit_child_recovery_and_duplicate_config_conflicts_are_preserved(self):
        from suite.selection import merge_execution_configs
        first = {**self.runtime, 'tasks': [copy.deepcopy(self.runtime['tasks'][0])]}
        child = {'taskId': self.parents[0]+'-m1', 'workingDirectory': '/app', 'agentTimeoutSec': 900,
                 'recovery': {'mode': 'service', 'bootstrap': {'files': [], 'env': {}, 'script': 'fixture'}}}
        second = {**self.runtime, 'tasks': [copy.deepcopy(first['tasks'][0]), child]}
        a, b = self.root/'a.json', self.root/'b.json'
        atomic_json(a, first); atomic_json(b, second)
        merged = merge_execution_configs([a, b])
        self.assertEqual(merged['tasks'], [first['tasks'][0], child])
        self.assertEqual(p.case_settings(merged, self.parents[0])[child['taskId']]['recovery'], child['recovery'])
        self.assertEqual(p.case_settings(first, self.parents[0])[self.parents[0]],
                         p.case_settings(merged, self.parents[0])[self.parents[0]])
        second['tasks'][0]['agentTimeoutSec'] += 1; atomic_json(b, second)
        with self.assertRaisesRegex(ValueError, 'conflicting duplicate'):
            merge_execution_configs([a,b])
        atomic_json(b, {**first, 'cases_root': str(self.root/'different')})
        with self.assertRaisesRegex(ValueError, 'roots differ'):
            merge_execution_configs([a,b])
        atomic_json(b, {'historical_artifact_root': '/historical-report'})
        with self.assertRaisesRegex(ValueError, 'historical runtime report'):
            merge_execution_configs([b])

    def test_subset_missing_child_or_changed_preparation_cannot_validate(self):
        manifest = self.freeze_subset()
        manifest['tasks'].pop()
        atomic_json(self.root/'subset/manifest.json', manifest)
        with self.assertRaises(ValueError):
            p.validate_manifest(self.root/'subset/manifest.json')

    def test_worker_refuses_unselected_before_profile_or_trial(self):
        from suite import run
        from suite.state import reserve, load
        manifest = self.freeze_subset()
        output = self.root/'worker'
        path = self.root/'subset/manifest.json'
        attempt = reserve(output, self.parents[1], digest(path))
        # Even a forged row cannot evade the explicit selected-parent guard.
        manifest['tasks'].append({'taskId': self.parents[1], 'parent_id': self.parents[1]})
        with patch.object(run, 'GLOBAL_LOCK', self.root/'model.lock'), \
             patch.object(run, 'validate_manifest', return_value=manifest), \
             patch('suite.agent._validate_profile') as profile, \
             patch.object(run, 'harbor_trial') as trial:
            run.worker(path, attempt)
        profile.assert_not_called(); trial.assert_not_called()
        self.assertEqual(load(attempt/'terminal.json')['error_type'], 'ValueError')
        self.assertIsNone(load(attempt/'terminal.json')['reward'])
        forbidden_output = self.root/'never-created'
        with self.assertRaises(ValueError):
            run.schedule(path, self.parents[1], forbidden_output,
                         validate=lambda path: p.load(path))
        self.assertFalse(forbidden_output.exists())

    def test_report_retains_full_denominators_and_rejects_excluded_records(self):
        from suite.report import build_report
        from suite.tests.test_report import result
        manifest = self.freeze_subset()
        rows = [result(kind=t['kind'], case_id=t['taskId'],
                       experiment_id=manifest['experiment_id']) for t in manifest['tasks']]
        for row in rows:
            row['hashes']['source'] = manifest['adapter']['sha256']
            row['hashes']['runtime'] = manifest['runtime']['binarySha256']
        report = build_report(p.load(self.catalog), rows, manifest)
        self.assertEqual(report['layers']['official']['expected'], 89)
        self.assertEqual(report['layers']['diagnostic']['expected'], 356)
        self.assertEqual(report['selected_execution']['expected'], 5)
        self.assertTrue(report['selected_execution']['complete'])
        self.assertEqual(report['experiment_scope'], manifest['scope'])
        rows.append(result(case_id=self.parents[1], experiment_id=manifest['experiment_id']))
        with self.assertRaisesRegex(ValueError, 'outside selected'):
            build_report(p.load(self.catalog), rows, manifest)

    def test_selected_stale_runtime_or_resource_receipt_is_rejected(self):
        path = self.cases/self.parents[0]/'preparation.json'
        original = p.load(path)
        mutations = []
        stale = copy.deepcopy(original); stale['scheduler_sha256'] = '0'*64; mutations.append(stale)
        binary = copy.deepcopy(original); binary['runtime_identity']['binary_sha256'] = '0'*64; mutations.append(binary)
        settings = copy.deepcopy(original); settings['case_settings'][self.parents[0]]['agentTimeoutSec'] += 1; mutations.append(settings)
        for value in mutations:
            atomic_json(path, value)
            # Each rejection is before any manifest is emitted; distinct output
            # folders avoid mistaking an old partial folder for the failure.
            with self.assertRaises(ValueError):
                p.freeze_ready_subset(self.catalog, self.cases, self.root/'runtime.json',
                                      self.root/('rejected-'+str(len(list(self.root.glob('rejected-*'))))), self.selection())
        self.assertFalse(list(self.root.glob('rejected-*/manifest.json')))

    def test_report_keeps_compatibility_variant_separate_and_binds_its_descriptor(self):
        from suite.report import build_report
        from suite.tests.test_report import result
        descriptor = {'id': 'build-cython-ext-compat-planarity06',
                      'contract_sha256': 'a'*64, 'image': 'vw-build-cython-compat@sha256:'+'a'*64,
                      'counts_as_original_official': False,
                      'files': {name: 'a'*64 for name in ('contract.json','Dockerfile','constraints.txt')}}
        manifest = self.freeze_subset()
        catalog = p.load(self.catalog)
        catalog['tasks'][0]['id'] = 'build-cython-ext'
        manifest['scope']['selected'] = ['build-cython-ext']
        for task in manifest['tasks']:
            task['taskId'] = task['taskId'].replace(self.parents[0], 'build-cython-ext')
            task['parent_id'] = 'build-cython-ext'
        manifest['tasks'][0]['kind'] = 'compatibility_variant'
        manifest['tasks'][0]['environment_variant'] = descriptor
        row = result('compatibility_variant', 'build-cython-ext',
                     experiment_id=manifest['experiment_id'], environment_variant=descriptor)
        row['hashes']['source'] = manifest['adapter']['sha256']
        row['hashes']['runtime'] = manifest['runtime']['binarySha256']
        report = build_report(catalog, [row], manifest)
        self.assertEqual(report['layers']['official']['observed'], 0)
        self.assertEqual(report['layers']['compatibility_variant']['observed'], 1)
        self.assertEqual(report['layers']['official']['expected'], 89)
        row['environment_variant'] = {**descriptor, 'contract_sha256': '0'*64}
        with self.assertRaises(ValueError):
            build_report(catalog, [row], manifest)

    def test_repeated_runtime_inputs_freeze_without_rewriting_or_overwriting_existing_output(self):
        a,b = self.root/'one.json', self.root/'two.json'
        atomic_json(a, {**self.runtime, 'tasks': [self.runtime['tasks'][0]]})
        atomic_json(b, {**self.runtime, 'tasks': [self.runtime['tasks'][1]]})
        before = {path:path.read_bytes() for path in (a,b)}
        selection = self.selection()
        manifest = p.freeze_ready_subset(self.catalog, self.cases, [a,b], self.root/'merged', selection)
        self.assertEqual(len(manifest['tasks']), 5)
        self.assertTrue(all(path.read_bytes()==data for path,data in before.items()))
        frozen = (self.root/'merged/manifest.json').read_bytes()
        with self.assertRaises(FileExistsError):
            p.freeze_ready_subset(self.catalog, self.cases, [a,b], self.root/'merged', selection)
        self.assertEqual((self.root/'merged/manifest.json').read_bytes(), frozen)

    def test_report_cli_emits_scope_but_does_not_relax_original_full_acceptance(self):
        import json
        from suite.report import main
        from suite.tests.test_report import result
        manifest = self.freeze_subset()
        history = self.root/'history.jsonl'
        rows=[result(kind=t['kind'], case_id=t['taskId'],experiment_id=manifest['experiment_id'])
              for t in manifest['tasks']]
        for row in rows:
            row['hashes']['source']=manifest['adapter']['sha256']
            row['hashes']['runtime']=manifest['runtime']['binarySha256']
        history.write_text('\n'.join(json.dumps(row) for row in rows))
        output=[]
        args=['--manifest', str(self.catalog), '--results', str(history),
              '--experiment', str(self.root/'subset/manifest.json')]
        self.assertEqual(main(args, emit=output.append),1)
        report=json.loads(output[-1])
        self.assertFalse(report['accepted'])
        self.assertTrue(report['selected_execution']['complete'])
        self.assertIn('experiment_sha256', report['sources'])
        self.assertEqual(main(args+['--audit-only'], emit=output.append),0)

    def test_report_refuses_same_experiment_id_with_wrong_runtime_or_adapter(self):
        from suite.report import build_report
        from suite.tests.test_report import result
        manifest=self.freeze_subset()
        row=result(case_id=self.parents[0],experiment_id=manifest['experiment_id'])
        row['hashes']['source']=manifest['adapter']['sha256']
        row['hashes']['runtime']=manifest['runtime']['binarySha256']
        for key in ('runtime','source'):
            changed=copy.deepcopy(row);changed['hashes'][key]='0'*64
            with self.assertRaises(ValueError):build_report(p.load(self.catalog),[changed],manifest)
