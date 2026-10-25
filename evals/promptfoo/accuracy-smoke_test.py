"""Offline tests: no credentials, Cargo, Gateway or upstream model requests."""
import importlib.util
import json
from pathlib import Path
import unittest
import tempfile

SPEC = importlib.util.spec_from_file_location('smoke', Path(__file__).with_name('accuracy-smoke.py'))
smoke = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(smoke)


class SmokeTests(unittest.TestCase):
    def test_relay_rejects_wrong_path_model_and_budget_before_upstream(self):
        gate = smoke.ModelGate('deepseek-v4-flash-0731')
        gate.enabled = True
        for path, body in [('/v1/models', {}), ('/v1/chat/completions?x=1', {}),
                           ('/v1/chat/completions', {'model': 'other', 'messages': []})]:
            with self.assertRaises(ValueError):
                gate.accept(path, json.dumps(body).encode())
        self.assertEqual(gate.calls, 0)
        gate.enabled = True
        good = json.dumps({'model': gate.model, 'messages': [{'role': 'user', 'content': 'x'}]}).encode()
        for _ in range(24):
            gate.accept('/v1/chat/completions', good)
            gate.finish(True)
        with self.assertRaises(ValueError):
            gate.accept('/v1/chat/completions', good)

    def test_relay_failure_closes_channel_without_retry(self):
        gate = smoke.ModelGate('m')
        gate.enabled = True
        gate.accept('/v1/chat/completions', b'{"model":"m","messages":[]}')
        with self.assertRaises(ValueError):
            gate.accept('/v1/chat/completions', b'{"model":"m","messages":[]}')
        gate.finish(False)
        with self.assertRaises(ValueError):
            gate.accept('/v1/chat/completions', b'{"model":"m","messages":[]}')
        self.assertEqual(gate.calls, 1)

    def test_relay_disallows_calls_until_admission(self):
        gate = smoke.ModelGate('m')
        with self.assertRaises(ValueError):
            gate.accept('/v1/chat/completions', b'{"model":"m","messages":[]}')

    def test_catalogue_must_resolve_exact_model_and_relay_before_admission(self):
        api = 'http://127.0.0.1:40001/v1'
        with self.assertRaises(RuntimeError):
            smoke.check_model_catalogue({'all': []}, api)
        provider = {'id': 'token-plan', 'models': {smoke.MODEL: {'api': {'id': smoke.MODEL, 'url': api}}}}
        smoke.check_model_catalogue({'all': [provider]}, api)
        provider['models'][smoke.MODEL]['api']['url'] = 'https://other.example/v1'
        with self.assertRaises(RuntimeError):
            smoke.check_model_catalogue({'all': [provider]}, api)

    def test_settlement_reaps_gateway_even_if_chat_error_has_no_session(self):
        original_results = smoke.RESULTS
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            smoke.RESULTS = root / 'results'
            smoke.RESULTS.mkdir()
            work = root / 'work'
            work.mkdir()
            (work / 'notes.txt').write_bytes(b'keep bytes\r\n')
            (work / 'settings.ini').write_bytes(b'wrong')
            runtime = root / 'runtime'
            (runtime / 'forbidden').mkdir(parents=True)
            supervisor = smoke.Supervisor()
            supervisor.runtime = runtime
            supervisor.work = str(work)
            supervisor.proof = {'digest': 'proof'}
            supervisor.evidence.update(attemptId='a', workRootIdentity={
                'dev': work.stat().st_dev, 'ino': work.stat().st_ino})
            stopped = []
            supervisor.stop = lambda: stopped.append(True)
            try:
                result = supervisor.settle('a', str(work), supervisor.proof)
                self.assertTrue(result['writersExited'])
                self.assertEqual(stopped, [True])
                self.assertTrue((smoke.RESULTS / 'artifacts/settings.ini').exists())
            finally:
                smoke.RESULTS = original_results

    def test_profile_only_opens_exact_relay_and_denies_fork(self):
        profile = smoke.profile('/private/tmp/run', '/private/tmp/work', 43210, 43211)
        self.assertIn('(remote ip "localhost:43211")', profile)
        self.assertNotIn('(allow process-fork)', profile)
        self.assertNotIn('(allow network-outbound)', profile)
        self.assertNotIn('(subpath "/private/tmp")', profile)
        self.assertIn('(deny default)', profile)


if __name__ == '__main__':
    unittest.main()
