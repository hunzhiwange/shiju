"""Conservative projection of reviewed compatible interfaces into runtime facts."""
import unittest
from build_model_protocol_catalog import build, operation


class ProtocolCatalogProjectionTests(unittest.TestCase):
    def test_reviewed_compatible_roots_load_sdk_only_catalogs(self):
        providers = build()['providers']
        for provider, base in [('vercel', 'https://ai-gateway.vercel.sh/v1'), ('cohere', 'https://api.cohere.ai/compatibility/v1')]:
            with self.subTest(provider=provider):
                self.assertEqual(providers[provider]['default_api'], base)
                endpoints = providers[provider]['deployments'][base]['protocols']
                self.assertTrue(endpoints)
                self.assertTrue(any(endpoint['protocol'] == 'chat' for endpoint in endpoints))

    def test_documented_loopback_operations_do_not_enable_remote_plaintext(self):
        self.assertTrue(operation('chat', 'http://localhost:8080/v1/chat/completions'))
        self.assertFalse(operation('chat', 'http://remote.example/v1/chat/completions'))
        self.assertFalse(operation('chat', 'http://localhost.evil.example/v1/chat/completions'))

    def test_unknown_authentication_is_not_inferred_from_sdk_tokens(self):
        from build_model_protocol_catalog import authentication
        self.assertIsNone(authentication('unknown', 'messages', 'ANTHROPIC_AUTH_TOKEN'))
