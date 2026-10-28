"""The second approved variant's metadata/proof protocol; no package operations."""
import asyncio
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import AsyncMock, patch

from suite import variants
from suite.state import atomic_json, digest, load, tree

PARENT = 'mcmc-sampling-stan'
ID = 'mcmc-sampling-stan-compat-cmake3283'
APPROVED_CLOSURE_SHA256 = '5fb755bed7fb032a15d63c0eb495a19b2253e345db2181c621190cd5d416cebb'


class CMakeVariantTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.cases = Path(self.temp.name)/'cases'
        self.root = self.cases/PARENT/'variant'; self.root.mkdir(parents=True)
        self.assertEqual(hashlib.sha256(json.dumps(variants.CMAKE_PACKAGES,sort_keys=True,
                         separators=(',',':')).encode()).hexdigest(),APPROVED_CLOSURE_SHA256)
        lock={'base':'alexgshaw/mcmc-sampling-stan@sha256:073fc36a3347b4f6b43bd6e98339542c41ed531c771af8bee40ba10f00cfabaf',
              'existing_packages_upgraded':[],
              'packages':[{'package':name,**row} for name,row in variants.CMAKE_PACKAGES.items()]}
        atomic_json(self.root/'deb-lock.json',lock)
        (self.root/'SHA256SUMS').write_text(''.join(row['sha256']+'  '+row['filename']+'\n'
                                         for row in variants.CMAKE_PACKAGES.values()))
        (self.root/'Dockerfile').write_text('FROM '+lock['base']+'\nRUN true\n')
        atomic_json(self.root/'base-packages.json', {'base-fixture': {'version':'1','architecture':'amd64'}})
        self.contract={'schema_version':1,'variant_id':ID,'original_parent_id':PARENT,
            'counts_as_original_official':False,'dependency_phase':'image_build_before_agent',
            'base_image':lock['base'],'image':'vw-mcmc-stan-cmake-compat@sha256:'+'a'*64,
            'cmake_version':'3.28.3','dockerfile_sha256':digest(self.root/'Dockerfile'),
            'packages_lock_sha256':digest(self.root/'deb-lock.json'),
            'sha256sums_sha256':digest(self.root/'SHA256SUMS'),
            'base_packages_sha256':digest(self.root/'base-packages.json')}
        atomic_json(self.root/'contract.json',self.contract)
        self.patch=patch.object(variants,'CASES',self.cases);self.patch.start();self.addCleanup(self.patch.stop)

    def descriptor(self):
        return variants.resolve(PARENT,{'id':ID,'contract_sha256':digest(self.root/'contract.json')})

    def test_only_approved_cmake_closure_resolves_without_installing_anything(self):
        value=self.descriptor()
        self.assertFalse(value['counts_as_original_official'])
        self.assertEqual(set(value['files']),{'contract.json','Dockerfile','deb-lock.json','SHA256SUMS','base-packages.json'})
        self.assertFalse((self.root/'cmake-debs').exists())

    def test_package_version_hash_or_extra_dependency_cannot_be_reauthorized_by_rehashing_contract(self):
        original=load(self.root/'deb-lock.json')
        for change in ('version','sha256','extra'):
            value=copy.deepcopy(original)
            if change=='extra': value['packages'].append({**value['packages'][0], 'package':'unapproved'})
            else: value['packages'][0][change]='changed'
            atomic_json(self.root/'deb-lock.json',value)
            contract={**self.contract,'packages_lock_sha256':digest(self.root/'deb-lock.json')}
            atomic_json(self.root/'contract.json',contract)
            with self.assertRaises(ValueError): self.descriptor()

    def test_read_only_before_agent_probe_matches_exact_packages_and_unchanged_base_inventory(self):
        import contextlib,io
        descriptor=self.descriptor()
        expected=variants.expected_facts(descriptor)
        inventory=load(self.root/'base-packages.json')|expected['packages']
        lines=''.join(name+'\t'+row['version']+'\t'+row['architecture']+'\n' for name,row in inventory.items())
        commands=[]
        def output(command):
            commands.append(command)
            if command[0]=='dpkg-query': return lines.encode()
            if command==['cmake','--version']: return b'cmake version 3.28.3\n'
            self.fail('unexpected non-read-only command')
        stream=io.StringIO()
        with patch('subprocess.check_output',side_effect=output),contextlib.redirect_stdout(stream):
            exec(variants.probe_code(descriptor),{})
        facts=json.loads(stream.getvalue())
        self.assertEqual(facts,expected)
        receipt={'environment_variant':descriptor,'phase':'before_agent','facts':facts}
        variants.check_start(receipt,descriptor)
        for changed in ({**facts,'cmake':'cmake version 3.29.0'},
                        {**facts,'base_inventory_sha256':'0'*64},
                        {**facts,'packages':{}}):
            with self.assertRaises(ValueError):variants.check_start({**receipt,'facts':changed},descriptor)
        self.assertEqual(len(commands),2)

    def test_cmake_variant_materialization_preserves_original_bytes_and_formal_kind(self):
        from suite.scoring import experiment_record
        from suite.contract import validate_record
        from datetime import datetime,timezone
        descriptor=self.descriptor()
        official=Path(__file__).resolve().parents[3]/'promptfoo/results/qwen38-suite/source/tasks'/PARENT
        before=tree(official)
        package=variants.materialize(official,Path(self.temp.name)/PARENT,descriptor)
        self.assertEqual(tree(package),variants.execution_files(official,descriptor))
        self.assertEqual(tree(official),before)
        task={'taskId':PARENT,'kind':'compatibility_variant','environment_variant':descriptor,'files':{}}
        row=experiment_record({'experiment_id':'synthetic','adapter':{'sha256':'a'*64},
            'runtime':{'binarySha256':'b'*64}},task,
            {'attempt_id':'one','started_at':datetime.now(timezone.utc).isoformat()}, {},'c'*64)
        validate_record(row)
        for changed in ({**row,'case_id':'build-cython-ext'},{**row,'kind':'official'}):
            with self.assertRaises(ValueError):validate_record(changed)

    def test_shared_backend_selects_second_variant_and_records_only_valid_pre_agent_facts(self):
        from harbor.environments.base import ExecResult
        from harbor.environments.docker.docker import DockerEnvironment
        from harbor.models.task.config import EnvironmentConfig
        from harbor.models.trial.paths import TrialPaths
        from suite.docker import SuiteDockerEnvironment
        descriptor=self.descriptor();root=Path(self.temp.name)
        envdir=root/'env';envdir.mkdir();(envdir/'Dockerfile').write_text('FROM cached\n')
        paths=TrialPaths(trial_dir=root/'trial');paths.mkdir()
        env=SuiteDockerEnvironment(environment_dir=envdir,environment_name='cmake',session_id='synthetic',
            trial_paths=paths,task_env_config=EnvironmentConfig(cpus=4,memory_mb=8192,docker_image=descriptor['image']),
            environment_variant=descriptor)
        facts=variants.expected_facts(descriptor)
        with patch.object(DockerEnvironment,'start',new=AsyncMock()) as start, \
             patch.object(env,'exec',new=AsyncMock(return_value=ExecResult(stdout=json.dumps(facts),stderr=None,return_code=0))):
            asyncio.run(env.start());start.assert_awaited_once()
        variants.check_start(load(paths.trial_dir/'environment-variant.json'),descriptor)
        env.task_env_config.docker_image='not-authorized'
        with patch.object(DockerEnvironment,'start',new=AsyncMock()) as start:
            with self.assertRaises(ValueError):asyncio.run(env.start())
            start.assert_not_awaited()

    def test_standard_entry_routes_oracle_nop_and_model_agent_to_second_variant(self):
        from suite.scoring import harbor_trial
        from suite.report import build_report
        from suite.tests.test_report import CATALOG,result
        descriptor=self.descriptor()
        official=Path(__file__).resolve().parents[3]/'promptfoo/results/qwen38-suite/source/tasks'/PARENT
        package=variants.materialize(official,Path(self.temp.name)/PARENT,descriptor)
        configs=[]
        class Result:
            def model_dump(self,**kwargs):return {}
        class Trial:
            async def run(self):return Result()
        async def create(config):configs.append(config);return Trial()
        with patch('harbor.trial.trial.Trial.create',side_effect=create):
            for agent in ('oracle','nop','suite.agent:SuiteAgent'):
                asyncio.run(harbor_trial(package,Path(self.temp.name)/'trials','synthetic',agent,
                                        kwargs={},environment_variant=descriptor))
        for config in configs:
            self.assertEqual(config.environment.kwargs,{'environment_variant':descriptor})
            self.assertEqual(config.environment.import_path,'suite.docker:SuiteDockerEnvironment')
        report=build_report(CATALOG,[result('compatibility_variant',PARENT,environment_variant=descriptor)])
        self.assertEqual(report['layers']['official']['observed'],0)
        self.assertEqual(report['layers']['compatibility_variant']['observed'],1)
