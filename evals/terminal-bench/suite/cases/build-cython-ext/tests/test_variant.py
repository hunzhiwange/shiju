"""The authorized variant must never overwrite or impersonate the original."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('cython_variant', ROOT / 'variant.py')
variant = importlib.util.module_from_spec(spec)
spec.loader.exec_module(variant)
OFFICIAL = ROOT.parents[4] / 'evals/promptfoo/results/qwen38-suite/source/tasks/build-cython-ext'


class VariantTests(unittest.TestCase):
    def test_rejects_wrong_wheel_before_creating_package(self):
        with tempfile.TemporaryDirectory() as t:
            wheel = Path(t) / 'bad.whl'
            wheel.write_bytes(b'incorrect artifact')
            with self.assertRaisesRegex(ValueError, 'wheel'):
                variant.materialize(OFFICIAL, wheel, Path(t) / 'package')
            self.assertFalse((Path(t) / 'package').exists())

    def test_rejects_source_drift_before_creating_package(self):
        with tempfile.TemporaryDirectory() as t:
            official = Path(t) / 'official'
            shutil.copytree(OFFICIAL, official)
            (official / 'instruction.md').write_text('changed prompt')
            with self.assertRaisesRegex(ValueError, 'official'):
                variant.validate_official(official)

    def test_contract_cannot_count_as_original_or_change_pins(self):
        original = json.loads((ROOT / 'variant/contract.json').read_text())
        for key, value in [('counts_as_original_official', True), ('planarity_version', '1.0.0'),
                           ('dependency_phase', 'verifier'), ('numpy_version', '2.0.0')]:
            with self.subTest(field=key), self.assertRaises(ValueError):
                variant.validate_contract({**original, key: value})
