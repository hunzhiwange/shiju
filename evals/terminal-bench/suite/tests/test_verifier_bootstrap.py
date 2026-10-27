"""Pinned caches may enter only the verifier phase, outside task artifacts."""
import hashlib
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest

from suite.scoring import bootstrap_verifier


class BootstrapTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.script = self.root / 'setup.sh'
        self.script.write_text('#!/bin/sh\nexit 0\n')
        self.policy = {'files': [{'source': str(self.script), 'target': 'setup.sh',
                                  'sha256': hashlib.sha256(self.script.read_bytes()).hexdigest()}],
                       'script': 'setup.sh', 'env': {'UV_PYTHON_DOWNLOADS': 'never'}}
        self.uploads, self.commands = [], []
        async def upload(source, target):
            self.uploads.append((str(source), target))
        async def execute(**kwargs):
            self.commands.append(kwargs)
            return SimpleNamespace(return_code=0, stdout='', stderr='')
        self.environment = SimpleNamespace(upload_file=upload, exec=execute)

    async def test_verified_cache_is_confined_and_receipted(self):
        self.policy['env'].update(UV_FIND_LINKS='/opt/vw-verifier-deps', UV_OFFLINE='1')
        env = await bootstrap_verifier(self.environment, self.policy, self.root)
        self.assertEqual(env, self.policy['env'])
        self.assertEqual(self.uploads[0][1], '/opt/vw-verifier-deps/setup.sh')
        self.assertIn('sha256sum -c', self.commands[-1]['command'])
        self.assertTrue((self.root / 'bootstrap.json').exists())

    async def test_hash_drift_denies_before_upload(self):
        self.script.write_text('changed')
        with self.assertRaises(ValueError):
            await bootstrap_verifier(self.environment, self.policy, self.root)
        self.assertFalse(self.uploads)

    async def test_unsafe_target_or_environment_denies_before_upload(self):
        for target in ['../tests/check.py', '/app/output', 'nested/../../tests', '']:
            self.policy['files'][0]['target'] = target
            with self.assertRaises(ValueError):
                await bootstrap_verifier(self.environment, self.policy, self.root)
        self.policy['files'][0]['target'] = 'setup.sh'
        self.policy['env']['LD_PRELOAD'] = 'unsafe'
        with self.assertRaises(ValueError):
            await bootstrap_verifier(self.environment, self.policy, self.root)
        self.assertFalse(self.uploads)

    async def test_failed_bootstrap_cannot_be_scored(self):
        async def fail(**kwargs):
            return SimpleNamespace(return_code=0 if kwargs['command'].startswith('mkdir') else 1,
                                   stdout='', stderr='setup failed')
        self.environment.exec = fail
        with self.assertRaises(ValueError):
            await bootstrap_verifier(self.environment, self.policy, self.root)

    async def test_duplicate_target_or_undeclared_script_denies_before_upload(self):
        self.policy['files'] *= 2
        with self.assertRaises(ValueError):
            await bootstrap_verifier(self.environment, self.policy, self.root)
        self.policy['files'] = self.policy['files'][:1]
        self.policy['script'] = 'missing.sh'
        with self.assertRaises(ValueError):
            await bootstrap_verifier(self.environment, self.policy, self.root)
        self.assertFalse(self.uploads)
