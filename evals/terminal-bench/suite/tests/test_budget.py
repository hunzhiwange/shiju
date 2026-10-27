"""Interop with the actual frozen Rust trust identities; no model/network use."""
import json
import unittest
from suite.budget import bundle, REQUIRED


class BudgetTests(unittest.TestCase):
    def test_trust_matches_native_catalog_and_does_not_grant_global_trust(self):
        files, config = bundle('/tmp/budget-probe', 'probe')
        store = json.loads(files['config/plugin-hook-trust.json'])
        # Captured from GET /v1/hooks on the newly built frozen ELF.
        self.assertEqual(store['trusted_hashes'], sorted([
            'afafc4e1311501ac089778e8addc65b3355c17d53fd3552b3f0060bda4406be1',
            '76538a986928d875fffcbd44507aacdc9b289ebaf7b97f1d125d0ed3155bbc42']))
        self.assertEqual(config['hooks']['required'], REQUIRED)
        self.assertEqual(config['reliability']['provider_retries'], 0)
        self.assertTrue(all(not name.startswith('/') and '..' not in name for name in files))

    def test_new_attempt_gets_distinct_gate_identity_and_initial_counters(self):
        a, _ = bundle('/tmp/a', 'one')
        b, _ = bundle('/tmp/b', 'two')
        self.assertNotEqual(a['config/plugin-hook-trust.json'], b['config/plugin-hook-trust.json'])
        self.assertEqual(a['budget/state'], 'attempt one\nmodels 0\ntools 0\n')
        self.assertEqual(a['budget/state.lock'], '')
        for root, attempt in [('relative', 'ok'), ('/tmp/../external', 'ok'), ('/tmp/ok', 'bad\nid')]:
            with self.assertRaises(ValueError):
                bundle(root, attempt)
