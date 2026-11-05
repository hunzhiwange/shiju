"""Contracts for the private upstream fixture, never supplier-support evidence."""
import json
from pathlib import Path
import tempfile
import unittest
from urllib.error import HTTPError
from urllib.request import Request, urlopen

from responses_fixture import ResponsesFixture
from verify_production import successful


class FixtureContracts(unittest.TestCase):
    def test_no_requests_wrong_route_auth_or_model_cannot_pass(self):
        flow = dict.fromkeys(('new', 'second_turn', 'cancel', 'restart', 'switch', 'isolated'), True)
        flow['mode'] = 'cli-local'
        record = {'model': 'acp-fixture-primary', 'authenticated': True,
                  'route': '/v1/responses', 'tool_results': 2}
        self.assertFalse(successful({'requests': [], 'errors': 0}, flow))
        self.assertTrue(successful({'requests': [record.copy() for _ in range(4)], 'errors': 0}, flow))
        for key, bad in [('authenticated', False), ('route', '/v1/chat/completions'), ('model', 'forbidden-native-model'), ('tool_results', 0)]:
            records = [dict(record, **{key: bad}) for _ in range(4)]
            self.assertFalse(successful({'requests': records, 'errors': 0}, flow))
        self.assertFalse(successful({'requests': [record] * 4, 'errors': 1}, flow))
        flow['mode'] = 'session-model'
        self.assertFalse(successful({'requests': [record] * 4, 'errors': 0}, flow))

    def test_real_http_rejects_unknown_route_credentials_and_model(self):
        for route, key, model in [('/v1/chat/completions', None, 'acp-fixture-primary'),
                                  ('/v1/responses', 'wrong', 'acp-fixture-primary'),
                                  ('/v1/responses', None, 'forbidden-native-model')]:
            with ResponsesFixture() as fixture:
                request = Request(fixture.url + route, json.dumps({'model': model, 'input': []}).encode(),
                                  {'Content-Type': 'application/json', 'Authorization': 'Bearer ' + (key or fixture.key)})
                with self.assertRaises(HTTPError):
                    urlopen(request, timeout=5)
                self.assertFalse(fixture.records)
                self.assertGreater(fixture.errors, 0)

    def test_two_distinct_file_reads_and_snapshot_redaction(self):
        with tempfile.TemporaryDirectory() as temp, ResponsesFixture() as fixture:
            path = Path(temp) / 'challenge.txt'
            path.write_text(fixture.challenge)
            fixture.read_path = str(path)
            self.assertNotEqual(fixture.read_path, fixture.second_read_path)
            self.assertEqual(Path(fixture.second_read_path).read_text(), fixture.challenge)
            body = {'model': 'acp-fixture-primary', 'input': [{'role': 'user', 'content': 'read'}],
                    'tools': [{'type': 'function', 'name': 'exec_command'}]}
            request = Request(fixture.url + '/v1/responses', json.dumps(body).encode(),
                              {'Content-Type': 'application/json', 'Authorization': 'Bearer ' + fixture.key})
            with urlopen(request, timeout=5) as response:
                output = json.load(response)['output'][0]
            self.assertEqual(output['type'], 'function_call')
            self.assertEqual(output['name'], 'exec_command')
            snapshot = json.dumps(fixture.snapshot())
            self.assertNotIn(fixture.key, snapshot)
            self.assertNotIn(fixture.control, snapshot)
            self.assertNotIn(fixture.challenge, snapshot)
            self.assertEqual(fixture.records[0]['tool_results'], 0)


if __name__ == '__main__':
    unittest.main()
