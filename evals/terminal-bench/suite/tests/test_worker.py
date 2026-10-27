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
        for raises in (False, True):
            with self.subTest(raises=raises), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                manifest_path = root / 'manifest.json'
                atomic_json(manifest_path, {'tasks': [{'taskId': 'parent', 'path': str(root)}],
                    'runtime_root': str(root)})
                manifest = load(manifest_path)
                profile = {'provider': {'api': 'http://127.0.0.1'}, 'auth': {'key': 'test-secret'}}
                profile_path = root / 'profile.json'
                atomic_json(profile_path, profile)
                public = hashlib.sha256(json.dumps(profile['provider'], sort_keys=True).encode()).hexdigest()
                atomic_json(root / 'experiment.json', {'provider_sha256': public})
                attempt = reserve(root, 'parent', digest(manifest_path))
                async def trial(*args, **kwargs):
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
                self.assertEqual(load(attempt / 'terminal.json')['state'], 'unknown')
                self.assertIsNone(load(attempt / 'terminal.json')['reward'])
