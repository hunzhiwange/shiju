"""Production variant boundaries without Docker or model requests."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from suite import preparation, variants
from suite.contract import validate_record
from suite.state import load, tree


class VariantTests(unittest.TestCase):
    def descriptor(self):
        path = variants.CASES / variants.PARENT / 'variant/contract.json'
        from suite.state import digest
        return variants.resolve(variants.PARENT, {'id': variants.ID, 'contract_sha256': digest(path)})

    def test_runtime_selects_and_freezes_only_authorized_variant(self):
        descriptor = self.descriptor()
        runtime = {'tasks': [{'taskId': variants.PARENT, 'workingDirectory': '/app',
                    'agentTimeoutSec': 900, 'environment_variant': descriptor}]}
        self.assertEqual(preparation.case_settings(runtime, variants.PARENT)[variants.PARENT]['environment_variant'], descriptor)
        with self.assertRaises(ValueError):
            variants.resolve('other-parent', descriptor)
        with self.assertRaises(ValueError):
            variants.resolve(variants.PARENT, {**descriptor, 'image': 'changed'})

    def test_materialized_package_preserves_official_source_and_rejects_drift(self):
        descriptor = self.descriptor()
        official = Path(__file__).resolve().parents[3] / 'promptfoo/results/qwen38-suite/source/tasks/build-cython-ext'
        before = tree(official)
        with tempfile.TemporaryDirectory() as t:
            package = variants.materialize(official, Path(t) / variants.PARENT, descriptor)
            self.assertEqual(tree(package), variants.execution_files(official, descriptor))
            self.assertEqual(variants.check_package(package, descriptor), descriptor)
            (package / 'instruction.md').write_text('altered')
            with self.assertRaisesRegex(ValueError, 'official task bytes'):
                variants.check_package(package, descriptor)
        self.assertEqual(tree(official), before)

    def test_formal_record_labels_variant_and_forbids_official_impersonation(self):
        from suite.scoring import experiment_record
        from datetime import datetime, timezone
        descriptor = self.descriptor()
        task = {'taskId': variants.PARENT, 'kind': 'compatibility_variant',
                'environment_variant': descriptor, 'files': {}}
        row = experiment_record({'experiment_id': 'variant', 'adapter': {'sha256': 'a'*64},
             'runtime': {'binarySha256': 'b'*64}}, task,
             {'attempt_id': 'one', 'started_at': datetime.now(timezone.utc).isoformat()}, {}, 'c'*64)
        self.assertEqual(row['environment_variant'], descriptor)
        self.assertEqual(row['kind'], 'compatibility_variant')
        with self.assertRaises(ValueError):
            validate_record({**row, 'kind': 'official'})
        with self.assertRaises(ValueError):
            validate_record({k:v for k,v in row.items() if k != 'environment_variant'})

    def test_missing_or_wrong_before_agent_dependencies_fail_closed(self):
        descriptor = self.descriptor()
        for receipt in [{}, {'environment_variant': descriptor, 'phase':'verifier', 'facts':{}},
                        {'environment_variant':descriptor,'phase':'before_agent','facts':{'planarity':'1.0.0'}}]:
            with self.subTest(receipt=receipt), self.assertRaises(ValueError):
                variants.check_start(receipt, descriptor)

    def test_shared_backend_checks_selected_image_and_dependencies_before_agent(self):
        import asyncio
        from unittest.mock import AsyncMock
        from harbor.environments.base import ExecResult
        from harbor.environments.docker.docker import DockerEnvironment
        from harbor.models.task.config import EnvironmentConfig
        from harbor.models.trial.paths import TrialPaths
        from suite.docker import SuiteDockerEnvironment
        descriptor = self.descriptor()
        with tempfile.TemporaryDirectory() as t:
            root = Path(t)
            envdir = root / 'environment'
            envdir.mkdir()
            (envdir / 'Dockerfile').write_text('FROM cached\n')
            paths = TrialPaths(trial_dir=root/'trial')
            paths.mkdir()
            env = SuiteDockerEnvironment(environment_dir=envdir, environment_name='variant',
                session_id='variant-test', trial_paths=paths,
                task_env_config=EnvironmentConfig(cpus=1,memory_mb=2048,docker_image=descriptor['image']),
                environment_variant=descriptor)
            facts = {'planarity':'0.6','numpy':'2.3.0','constraint':'/opt/vw-compat/constraints.txt',
                     'constraints_sha256':descriptor['files']['constraints.txt']}
            with patch.object(DockerEnvironment,'start',new=AsyncMock()) as start, \
                 patch.object(env,'exec',new=AsyncMock(return_value=ExecResult(stdout=json.dumps(facts),stderr=None,return_code=0))):
                asyncio.run(env.start())
                start.assert_awaited_once()
                variants.check_start(load(paths.trial_dir/'environment-variant.json'),descriptor)
            env.task_env_config.docker_image='wrong-image'
            with patch.object(DockerEnvironment,'start',new=AsyncMock()) as start:
                with self.assertRaises(ValueError):asyncio.run(env.start())
                start.assert_not_awaited()

    def test_report_never_counts_compatibility_score_as_official(self):
        from suite.tests.test_report import CATALOG, result
        from suite.report import build_report
        row=result('compatibility_variant',variants.PARENT,environment_variant=self.descriptor())
        report=build_report(CATALOG,[row])
        self.assertEqual(report['layers']['compatibility_variant']['scored'],1)
        self.assertEqual(report['layers']['official']['scored'],0)
        self.assertEqual(report['layers']['official']['missing'],89)


    def test_shared_entry_routes_calibration_and_model_agent_to_same_variant_backend(self):
        import asyncio
        from suite.scoring import harbor_trial
        descriptor = self.descriptor()
        official = Path(__file__).resolve().parents[3] / 'promptfoo/results/qwen38-suite/source/tasks/build-cython-ext'
        captured = []
        class Result:
            def model_dump(self, **kwargs): return {}
        class Trial:
            async def run(self): return Result()
        async def create(config):
            captured.append(config)
            return Trial()
        with tempfile.TemporaryDirectory() as t:
            package = variants.materialize(official, Path(t)/variants.PARENT, descriptor)
            with patch('harbor.trial.trial.Trial.create', side_effect=create):
                for agent in ['oracle', 'nop', 'suite.agent:SuiteAgent']:
                    asyncio.run(harbor_trial(package, Path(t)/'trials', 'variant', agent,
                                            kwargs={}, environment_variant=descriptor))
        for config in captured:
            self.assertEqual(config.environment.import_path, 'suite.docker:SuiteDockerEnvironment')
            self.assertEqual(config.environment.kwargs, {'environment_variant':descriptor})

    def test_changed_recipe_cannot_reuse_frozen_variant_descriptor(self):
        descriptor = self.descriptor()
        with tempfile.TemporaryDirectory() as t:
            cases = Path(t)/'cases'
            shutil.copytree(variants.CASES/variants.PARENT/'variant', cases/variants.PARENT/'variant')
            (cases/variants.PARENT/'variant/constraints.txt').write_text('planarity==1.0.0\n')
            with patch.object(variants,'CASES',cases), self.assertRaisesRegex(ValueError,'contract drift'):
                variants.resolve(variants.PARENT,descriptor)


from suite.tests import test_preparation as preparation_fixtures
from suite.state import atomic_json, digest
import shutil


class StandardWiringTests(unittest.TestCase):
    setUp = preparation_fixtures.PreparationTests.setUp
    materialize = preparation_fixtures.PreparationTests.materialize

    def configure_variant(self):
        # Synthetic 89-parent scaffold exercises routing, not authorization;
        # authorization against the real parent is tested separately above.
        descriptor = VariantTests.descriptor(self)
        parent = self.parents[0]
        current = self.current[parent]
        current['environment_variant'] = descriptor
        current['case_settings'][parent]['environment_variant'] = descriptor
        import hashlib
        current['official_files'] = tree(current['official'])
        current['execution_files'] = tree(current['official'])
        current['execution_files']['task.toml']['sha256'] = hashlib.sha256(
            ('[task]\nname = \"terminal-bench/' + variants.ID + '\"\n').encode()).hexdigest()
        def package(official, output, value):
            self.assertEqual(value, descriptor)
            shutil.copytree(official, output)
            (output/'task.toml').write_text('[task]\nname = "terminal-bench/' + variants.ID + '"\n')
            return output
        self.stack.enter_context(patch.object(variants,'materialize',side_effect=package))
        self.stack.enter_context(patch.object(variants,'check_package',return_value=descriptor))
        return parent, descriptor

    def preflight_variant(self):
        parent, descriptor = self.configure_variant()
        calls=[]
        facts={'planarity':'0.6','numpy':'2.3.0','constraint':'/opt/vw-compat/constraints.txt',
               'constraints_sha256':descriptor['files']['constraints.txt']}
        async def trial(task, directory, name, agent, recovery=None, **kwargs):
            is_parent=task.name==parent
            calls.append((task.name,kwargs))
            result={'task_name':'terminal-bench/'+variants.ID if is_parent else task.name,
                    'agent_info':{'name':agent},'verifier_result':{'rewards':{'reward':int(agent=='oracle')}},
                    'config':{'environment':{'import_path':'suite.docker:SuiteDockerEnvironment'}}}
            atomic_json(directory/name/'scoring/first.json',result['verifier_result'])
            if is_parent:
                atomic_json(directory/name/'environment-variant.json',{'phase':'before_agent',
                            'environment_variant':descriptor,'facts':facts})
                self.current[parent]['execution_files']=tree(task)
            return result
        with patch.object(preparation,'harbor_trial',side_effect=trial):
            preparation.preflight(self.catalog,self.root/'runtime.json',parent,self.root/'variant-preflight')
        self.assertEqual(len(calls),10)
        for task,kwargs in calls:
            self.assertEqual(kwargs,{'environment_variant':descriptor} if task==parent else {})
        preparation.verify_preparation(self.cases/parent/'preparation.json',self.current[parent],None)
        return parent,descriptor

    def test_standard_preflight_freeze_and_validation_preserve_variant_and_reject_relabel(self):
        parent,descriptor=self.preflight_variant()
        manifest=preparation.freeze(self.catalog,self.cases,self.root/'runtime.json',self.root/'frozen')
        self.assertEqual(manifest['tasks'][0]['kind'],'compatibility_variant')
        self.assertEqual(manifest['tasks'][0]['environment_variant'],descriptor)
        self.assertIn('/variants/',manifest['tasks'][0]['path'])
        self.assertEqual(preparation.validate_manifest(self.root/'frozen/manifest.json'),manifest)
        manifest['tasks'][0]['kind']='official'
        atomic_json(self.root/'frozen/manifest.json',manifest)
        with self.assertRaises(ValueError):preparation.validate_manifest(self.root/'frozen/manifest.json')

    def test_old_private_or_missing_start_receipt_cannot_admit_variant(self):
        parent,descriptor=self.preflight_variant()
        path=self.cases/parent/'preparation.json'
        value=load(path)
        value['calibration'][0]['result']['config']['environment']={'type':'docker','import_path':None}
        atomic_json(path,value)
        with self.assertRaisesRegex(ValueError,'standard pre-agent'):
            preparation.verify_preparation(path,self.current[parent],None)
        value['calibration'][0].pop('environment_variant_start')
        atomic_json(path,value)
        with self.assertRaisesRegex(ValueError,'standard pre-agent'):
            preparation.verify_preparation(path,self.current[parent],None)
