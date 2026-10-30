"""The approved upgrade policy preserves the original task and seven-package base."""
import contextlib
import io
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch

from suite import variants
from suite.state import atomic_json, digest, load, tree
from suite.tests import test_cmake_variant


class StanVariantTests(unittest.TestCase):
    def setUp(self):
        test_cmake_variant.CMakeVariantTests.setUp(self)
        self.contract['image'] = variants.STAN_BASE
        atomic_json(self.root / 'contract.json', self.contract)
        base_patch = patch.object(variants, 'STAN_BASE_CONTRACT', digest(self.root / 'contract.json'))
        base_patch.start(); self.addCleanup(base_patch.stop)
        self.stan = self.root.parent / 'stan-variant'
        self.stan.mkdir()
        (self.stan / 'Dockerfile').write_text(variants.STAN_RECIPE)
        self.new_contract = {
            'variant_id': variants.STAN_ID, 'original_parent_id': variants.CMAKE_PARENT,
            'counts_as_original_official': False, 'dependency_phase': 'image_build_before_agent',
            'base_image': variants.STAN_BASE, 'base_variant': variants.stan_base_descriptor(),
            'environment': {'R_REMOTES_UPGRADE': 'never'},
            'image': 'vw-mcmc-stan-remotes-never@sha256:' + 'b' * 64,
            'dockerfile_sha256': digest(self.stan / 'Dockerfile')}
        atomic_json(self.stan / 'contract.json', self.new_contract)

    def descriptor(self):
        return variants.resolve(variants.CMAKE_PARENT, {
            'id': variants.STAN_ID, 'contract_sha256': digest(self.stan / 'contract.json')})

    def test_formal_backend_requires_valid_new_variant_pre_agent_proof(self):
        test_cmake_variant.CMakeVariantTests.test_shared_backend_selects_second_variant_and_records_only_valid_pre_agent_facts(self)

    def test_standard_entry_routes_new_variant_and_keeps_official_results_separate(self):
        test_cmake_variant.CMakeVariantTests.test_standard_entry_routes_oracle_nop_and_model_agent_to_second_variant(self)

    def test_new_identity_preserves_original_task_and_cmake_variant(self):
        previous = tree(self.root)
        descriptor = self.descriptor()
        self.assertEqual(variants.parent_for(descriptor), variants.CMAKE_PARENT)
        official = Path(__file__).resolve().parents[3] / 'promptfoo/results/qwen38-suite/source/tasks' / variants.CMAKE_PARENT
        before = tree(official)
        package = variants.materialize(official, Path(self.temp.name) / 'task', descriptor)
        self.assertEqual(tree(package), variants.execution_files(official, descriptor))
        self.assertEqual(tree(official), before)
        self.assertEqual(tree(self.root), previous)

    def test_rehashed_recipe_cannot_install_packages_or_change_policy(self):
        for extra in ('RUN true\n', 'ENV R_REMOTES_UPGRADE=always\n', 'ENV OTHER=1\n'):
            (self.stan / 'Dockerfile').write_text(variants.STAN_RECIPE + extra)
            atomic_json(self.stan / 'contract.json', {
                **self.new_contract, 'dockerfile_sha256': digest(self.stan / 'Dockerfile')})
            with self.assertRaises(ValueError): self.descriptor()

    def test_rehashed_contract_cannot_change_base_policy_or_official_kind(self):
        for field, value in (('base_image', 'other'), ('base_variant', {}),
                             ('environment', {'R_REMOTES_UPGRADE': 'always'}),
                             ('counts_as_original_official', True)):
            atomic_json(self.stan / 'contract.json', {**self.new_contract, field: value})
            with self.assertRaises(ValueError): self.descriptor()

    def test_before_agent_probe_observes_environment_and_unchanged_inventory(self):
        descriptor = self.descriptor()
        facts = variants.expected_facts(descriptor)
        inventory = load(self.root / 'base-packages.json') | facts['packages']
        rows = ''.join(n + '\t' + r['version'] + '\t' + r['architecture'] + '\n'
                       for n, r in inventory.items())
        def output(command):
            if command[0] == 'dpkg-query': return rows.encode()
            if command == ['cmake', '--version']: return b'cmake version 3.28.3\n'
            self.fail('unexpected command')
        for policy in ('never', 'always', None):
            with patch.dict(os.environ, {}, clear=True):
                if policy is not None: os.environ['R_REMOTES_UPGRADE'] = policy
                stream = io.StringIO()
                with patch('subprocess.check_output', side_effect=output), contextlib.redirect_stdout(stream):
                    exec(variants.probe_code(descriptor), {})
            observed = json.loads(stream.getvalue())
            receipt = {'environment_variant': descriptor, 'phase': 'before_agent', 'facts': observed}
            if policy == 'never': variants.check_start(receipt, descriptor)
            else:
                with self.assertRaises(ValueError): variants.check_start(receipt, descriptor)

    def test_missing_or_symlinked_variant_file_is_rejected(self):
        recipe = self.stan / 'Dockerfile'
        recipe.unlink()
        with self.assertRaises(ValueError): self.descriptor()
        recipe.symlink_to(self.root / 'Dockerfile')
        with self.assertRaises(ValueError): self.descriptor()
