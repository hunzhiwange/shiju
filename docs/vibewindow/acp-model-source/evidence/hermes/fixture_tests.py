"""Hermes fixture compatibility must preserve model endpoint authentication."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from urllib.error import HTTPError
from urllib.request import Request, urlopen

spec = importlib.util.spec_from_file_location("hermes_acceptance", Path(__file__).with_name("verify.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class HermesFixtureTests(unittest.TestCase):
    def test_discovery_404_is_not_a_model_request_and_authentication_stays_required(self):
        with module.HermesFixture() as fixture:
            with self.assertRaises(HTTPError) as failure:
                urlopen(fixture.url + "/api/tags")
            self.assertEqual(failure.exception.code, 404)
            self.assertEqual(fixture.snapshot()["requests"], [])
            self.assertEqual(fixture.snapshot()["errors"], 0)
            with self.assertRaises(HTTPError):
                urlopen(fixture.url + "/v1/models")
            body = json.dumps({"model": fixture.models[0], "messages": [{"role": "user", "content": "probe"}]}).encode()
            with self.assertRaises(HTTPError):
                urlopen(Request(fixture.url + "/v1/chat/completions", body, {"Content-Type": "application/json"}))
            self.assertEqual(fixture.snapshot()["requests"], [])
            self.assertEqual(fixture.snapshot()["errors"], 2)

    def test_second_read_uses_distinct_real_file_with_same_challenge(self):
        with tempfile.TemporaryDirectory(prefix="acp-model-acceptance-") as directory, module.HermesFixture() as fixture:
            first = Path(directory) / "challenge.txt"
            first.write_text(fixture.challenge)
            fixture.read_path = str(first)
            second = Path(fixture.second_read_path)
            self.assertNotEqual(first, second)
            self.assertEqual(first.read_text(), second.read_text())
            self.assertEqual(second.stat().st_mode & 0o777, 0o600)


if __name__ == "__main__":
    unittest.main()
