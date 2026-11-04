"""Pi identity/isolation negatives; these tests never stand in for a real CLI run."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import urllib.request

from acp_model_source_fixture import ModelFixture
from acp_model_source_pi import ADAPTER_SHA256, PiIdentityError, package_identity, pi_identity
from verify_acp_model_source import isolated_environment

ROOT = Path(__file__).resolve().parents[1]


class PiBoundary(unittest.TestCase):
    def test_forged_matching_version_is_rejected_before_execution(self):
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary)
            (package / 'dist').mkdir()
            (package / 'package.json').write_text(json.dumps({'name': 'pi-acp', 'version': '0.0.22'}))
            executable = package / 'dist/index.js'
            sentinel = package / 'executed'
            executable.write_text('#!/bin/sh\ntouch "' + str(sentinel) + '"\necho 0.0.22\n')
            executable.chmod(0o700)
            with self.assertRaises(PiIdentityError):
                package_identity(executable, 'pi-acp', '0.0.22', ADAPTER_SHA256)
            self.assertFalse(sentinel.exists())

    def test_alternative_package_entry_is_rejected_even_with_matching_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary)
            (package / 'dist').mkdir()
            (package / 'package.json').write_text(json.dumps({'name': 'pi-acp', 'version': '0.0.22'}))
            executable = package / 'dist/other.js'
            executable.write_bytes(b'not a supplier CLI')
            digest = hashlib.sha256(executable.read_bytes()).hexdigest()
            with self.assertRaises(PiIdentityError):
                package_identity(executable, 'pi-acp', '0.0.22', digest)

    def test_manifest_cannot_select_a_different_supplier_version(self):
        entry = next(e for e in json.loads((ROOT / 'docs/vibewindow/acp-model-source/cli-versions.json').read_text())['entries'] if e['key'] == 'pi')
        entry['distribution']['release'] = 'latest'
        with self.assertRaises(PiIdentityError):
            pi_identity('/never/executed', entry, {})

    def test_http_detector_distinguishes_native_compaction_from_primary_calls(self):
        with ModelFixture() as fixture:
            fixture.pi_compaction = True
            base = {"model": fixture.models[0], "messages": [
                {"role": "system", "content": "You are a context summarization assistant."},
                {"role": "user", "content": "<conversation>read file</conversation> summarize"}]}
            for index in range(3):
                body = json.loads(json.dumps(base))
                if index == 1:
                    body['tools'] = [{'type': 'function', 'function': {'name': 'read', 'parameters': {'properties': {'path': {'type': 'string'}}}}}]
                if index == 2:
                    body['messages'][0]['content'] = 'You are a coding assistant.'
                request = urllib.request.Request(fixture.url + '/v1/chat/completions',
                    json.dumps(body).encode(), {'Authorization': 'Bearer ' + fixture.key, 'Content-Type': 'application/json'})
                with urllib.request.urlopen(request) as response:
                    response.read()
            records = fixture.snapshot()['requests']
            self.assertEqual([record['auxiliary'] for record in records], [True, False, False])
            self.assertEqual(fixture.snapshot()['errors'], 0)

    def test_native_and_acp_roots_are_private_without_home_substitution(self):
        entry = next(e for e in json.loads((ROOT / 'docs/vibewindow/acp-model-source/cli-versions.json').read_text())['entries'] if e['key'] == 'pi')
        for mode in ('session-model', 'cli-local'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary, ModelFixture() as fixture:
                directory = Path(temporary)
                environment = isolated_environment(entry, directory, fixture, mode)
                self.assertEqual(set(environment), {'PI_CODING_AGENT_DIR', 'PI_ACP_STATE_DIR'})
                self.assertEqual(environment['PI_CODING_AGENT_DIR'], str(directory))
                self.assertEqual(environment['PI_ACP_STATE_DIR'], str(directory / 'acp'))
                self.assertEqual((directory / 'acp').stat().st_mode & 0o777, 0o700)
                models = json.loads((directory / 'models.json').read_text())['providers']['session-provider']
                self.assertEqual(models['models'][0]['id'], 'forbidden-native-model' if mode == 'session-model' else fixture.models[0])
                self.assertEqual(models['apiKey'], 'offline-native-account-must-not-be-used' if mode == 'session-model' else fixture.key)
                self.assertEqual((directory / 'models.json').stat().st_mode & 0o777, 0o600)
                self.assertEqual(fixture.snapshot()['requests'], [])


if __name__ == '__main__':
    unittest.main()
