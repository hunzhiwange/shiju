"""Execute all positive/negative artifacts in independent production sandboxes."""
import json
from pathlib import Path
import unittest
from suite import diagnostics

ROOT = Path(__file__).resolve().parents[1]
CASE, _ = diagnostics.load_case(ROOT / 'case.json')


class MipsCases(unittest.TestCase):
    pass


def add_test(child, identity, files, expected):
    def test(self):
        self.assertEqual(diagnostics.run_checker(CASE['image'], ROOT, child, files), expected)
    setattr(MipsCases, f"test_{child['id']}_{identity.replace('-', '_')}", test)


for child in CASE['children']:
    add_test(child, 'gold', child['gold'], 0)
    add_test(child, 'nop', {}, 1)
    for negative in child['negatives']:
        add_test(child, negative['id'], negative['files'], 1)


if __name__ == '__main__':
    unittest.main()
