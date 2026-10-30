"""Real Nginx configuration and HTTP in fresh isolated fixture containers."""
import json
from pathlib import Path
import unittest
from suite import diagnostics

ROOT = Path(__file__).resolve().parents[1]


class NginxCases(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.case, _ = diagnostics.load_case(ROOT / 'case.json')


def add_test(child_id, identity, expected):
    def test(self):
        child = next(c for c in self.case['children'] if c['id'] == child_id)
        files = (child['gold'] if identity == 'gold' else {} if identity == 'nop' else
                 next(n['files'] for n in child['negatives'] if n['id'] == identity))
        self.assertEqual(diagnostics.run_checker(self.case['image'], ROOT, child, files), expected)
    setattr(NginxCases, f'test_{child_id}_{identity.replace("-", "_")}', test)


for child in json.loads((ROOT / 'case.json').read_text())['children']:
    for identity, expected in [('gold', 0), ('nop', 1)] + [(n['id'], 1) for n in child['negatives']]:
        add_test(child['id'], identity, expected)

if __name__ == '__main__':
    unittest.main()
