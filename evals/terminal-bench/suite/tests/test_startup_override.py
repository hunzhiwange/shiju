"""Only a hash-bound restoration of the original image CMD may reach Harbor."""
import asyncio
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from suite import scoring, preparation

BODY = b'services:\n  main:\n    command: null\n'


class StartupOverrideTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.environment = self.root / 'example/environment'
        self.environment.mkdir(parents=True)
        self.path = self.environment / 'restore-image-cmd.yaml'
        self.path.write_bytes(BODY)
        self.descriptor = {'source': self.path.name, 'sha256': hashlib.sha256(BODY).hexdigest()}
        self.patch = patch.object(scoring, 'CASES', self.root, create=True)
        self.patch.start()
        self.addCleanup(self.patch.stop)

    def test_only_trusted_canonical_hashed_override_is_accepted(self):
        self.assertEqual(scoring.checked_startup_override('example', self.descriptor), self.path)
        for changed in ({**self.descriptor, 'unknown': True},
                        {**self.descriptor, 'source': '../restore-image-cmd.yaml'},
                        {**self.descriptor, 'source': str(self.path)},
                        {**self.descriptor, 'source': 'a\\b.yaml'},
                        {**self.descriptor, 'sha256': '0' * 64}, None):
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                scoring.checked_startup_override('example', changed)
        for identity in ('../example', '/example', '.', '..', 'a/b'):
            with self.subTest(identity=identity), self.assertRaises(ValueError):
                scoring.checked_startup_override(identity, self.descriptor)

    def test_unknown_fields_services_and_compose_capabilities_are_rejected_even_with_correct_hash(self):
        for body in (b'services:\n  main:\n    command: echo changed\n',
                     BODY + b'    privileged: true\n', BODY + b'    volumes: [/tmp:/host]\n',
                     BODY + b'    ports: [80:80]\n', BODY + b'    network_mode: host\n',
                     BODY + b'  other:\n    image: attacker\n', BODY + b'unknown: true\n'):
            self.path.write_bytes(body)
            value = {**self.descriptor, 'sha256': hashlib.sha256(body).hexdigest()}
            with self.subTest(body=body), self.assertRaises(ValueError):
                scoring.checked_startup_override('example', value)

    def test_symlink_file_or_trusted_environment_is_rejected(self):
        outside = self.root / 'outside.yaml'
        outside.write_bytes(BODY)
        self.path.unlink()
        self.path.symlink_to(outside)
        with self.assertRaises(ValueError):
            scoring.checked_startup_override('example', self.descriptor)
        self.path.unlink()
        self.environment.rmdir()
        self.environment.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(ValueError):
            scoring.checked_startup_override('example', self.descriptor)

    def test_preparation_binds_override_and_rejects_byte_drift(self):
        runtime = {'tasks': [{'taskId': 'example', 'workingDirectory': '/app',
                             'agentTimeoutSec': 900, 'startup_override': self.descriptor}]}
        settings = preparation.case_settings(runtime, 'example')
        self.assertEqual(settings['example']['startup_override'], self.descriptor)
        self.path.write_bytes(BODY + b'# changed\n')
        with self.assertRaises(ValueError):
            preparation.case_settings(runtime, 'example')

    def test_harbor_receives_verified_overlay_and_undeclared_behavior_is_unchanged(self):
        captured = []
        overlay_bytes = []
        class Result:
            def model_dump(self, **kwargs):
                return {'task_name': 'example'}
        class Trial:
            def add_hook(self, *args):
                pass
            async def run(self):
                return Result()
        async def create(config):
            captured.append(config)
            overlay_bytes.append([path.read_bytes() for path in config.environment.extra_docker_compose])
            return Trial()
        with patch('harbor.trial.trial.Trial.create', side_effect=create):
            asyncio.run(scoring.harbor_trial(self.root/'example', self.root/'trials', 'plain', 'nop'))
            asyncio.run(scoring.harbor_trial(self.root/'example', self.root/'trials', 'restored', 'nop',
                                            startup_override=self.descriptor))
        self.assertEqual(captured[0].environment.extra_docker_compose, [])
        self.assertEqual(overlay_bytes, [[], [BODY]])
        self.assertNotEqual(captured[1].environment.extra_docker_compose, [self.path])
        self.assertEqual(captured[1].environment.extra_docker_compose[0].name, self.path.name)
        self.assertIsNone(captured[1].environment.override_cpus)
        self.assertIsNone(captured[1].environment.override_memory_mb)
        self.assertEqual(captured[1].verifier.import_path, 'suite.scoring:RecoveringVerifier')
        with patch('harbor.trial.trial.Trial.create') as create_mock:
            self.path.write_bytes(b'services: {}\n')
            with self.assertRaises(ValueError):
                asyncio.run(scoring.harbor_trial(self.root/'example', self.root/'trials', 'bad', 'nop',
                                                startup_override=self.descriptor))
            create_mock.assert_not_called()
