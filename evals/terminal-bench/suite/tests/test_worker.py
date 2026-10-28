"""Uncertain dispatched work must keep the global no-replay barrier."""
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from suite import run
from suite.state import atomic_json, digest, reserve, load


class WorkerTests(unittest.TestCase):
    def test_dispatch_exception_or_missing_cleanup_evidence_remains_unknown(self):
        for raises, declared, task_id in ((False, False, 'parent'), (True, False, 'parent'),
                                         (False, True, 'parent'), (True, True, 'parent'),
                                         (False, False, 'parent-m1'), (True, False, 'parent-m1'),
                                         (False, False, 'build-cython-ext')):
            with self.subTest(raises=raises, declared=declared, task_id=task_id), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                manifest_path = root / 'manifest.json'
                task = {'taskId': task_id, 'path': str(root)}
                descriptor = {'source': 'restore-image-cmd.yaml', 'sha256': 'f' * 64}
                if declared:
                    task['startup_override'] = descriptor
                variant = None
                if task_id == 'build-cython-ext':
                    variant = {'id': 'build-cython-ext-compat-planarity06', 'contract_sha256': 'a'*64}
                    task['environment_variant'] = variant
                tasks = [task]
                if task_id == 'parent-m1':
                    tasks.insert(0, {'taskId': 'parent', 'path': str(root), 'startup_override': descriptor})
                atomic_json(manifest_path, {'tasks': tasks,
                    'runtime_root': str(root)})
                manifest = load(manifest_path)
                profile = {'provider': {'api': 'http://127.0.0.1'}, 'auth': {'key': 'test-secret'}}
                profile_path = root / 'profile.json'
                atomic_json(profile_path, profile)
                public = hashlib.sha256(json.dumps(profile['provider'], sort_keys=True).encode()).hexdigest()
                atomic_json(root / 'experiment.json', {'provider_sha256': public})
                attempt = reserve(root, task_id, digest(manifest_path))
                captured = []
                async def trial(*args, **kwargs):
                    captured.append(kwargs)
                    if raises:
                        raise RuntimeError('uncertain transport')
                    return {'exception_info': {'exception_type': 'Interrupted'}}
                with patch.object(run, 'GLOBAL_LOCK', root / 'global.lock'), \
                     patch.object(run, 'validate_manifest', return_value=manifest), \
                     patch('suite.agent._validate_profile', return_value=profile), \
                     patch.dict(os.environ, {'VW_SUITE_PROFILE': str(profile_path)}), \
                     patch.object(run, 'experiment_record', return_value={}), \
                     patch.object(run, 'harbor_trial', side_effect=trial):
                    run.worker(manifest_path, attempt)
                expected = {'startup_override': descriptor} if declared else {}
                if variant is not None:
                    expected['environment_variant'] = variant
                self.assertEqual(captured, [expected])
                self.assertEqual(load(attempt / 'terminal.json')['state'], 'unknown')
                self.assertIsNone(load(attempt / 'terminal.json')['reward'])
