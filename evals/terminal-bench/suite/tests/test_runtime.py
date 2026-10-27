"""Runtime identity must fail closed before any container execution."""
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

from suite import runtime
from suite.runtime import verify_runtime


class RuntimeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / 'runtime').mkdir()
        self.binary = self.root / 'runtime/shiju'
        self.binary.write_bytes(b'\x7fELF' + bytes([2, 1, 1]) + bytes(9) + struct.pack('<HHI', 2, 62, 1) + bytes(40))
        (self.root / 'source').mkdir()
        source_hash = patch('suite.runtime.SOURCE_SHA256', runtime.source_digest(self.root / 'source'))
        source_hash.start()
        self.addCleanup(source_hash.stop)
        (self.root / 'source.json').write_text(json.dumps({
            'baseCommit': runtime.BASE_COMMIT, 'fixedSourceSha256': runtime.SOURCE_SHA256,
            'onlyProductionPatch': 'retry-fix.diff'}))
        repair = b'+        Ok(AssistantError::APIError { is_retryable: false, .. })'
        (self.root / 'retry-fix.diff').write_bytes(repair)
        (self.root / 'runtime-source.diff').write_bytes(repair)
        build = {'binarySha256': runtime.sha256(self.binary), 'sourceBefore': runtime.SOURCE_SHA256,
                 'sourceAfter': runtime.SOURCE_SHA256, 'sourceCommit': runtime.BASE_COMMIT,
                 'CFLAGS_x86_64_unknown_linux_gnu': '-DSHA1DC_FORCE_ALIGNED_ACCESS=1',
                 'commands': [{'exitCode': 0, 'command': ['cargo', 'build', '-p', 'vw-cli', '--bin',
                              'shiju', '--target', 'x86_64-unknown-linux-gnu', '--locked']}]}
        (self.root / 'build-evidence.json').write_text(json.dumps(build))
        binary_hash = patch('suite.runtime.BINARY_SHA256', runtime.sha256(self.binary))
        binary_hash.start()
        self.addCleanup(binary_hash.stop)

    def test_wrong_architecture_rejected(self):
        data = bytearray(self.binary.read_bytes())
        data[18:20] = struct.pack('<H', 183)
        self.binary.write_bytes(data)
        with self.assertRaisesRegex(ValueError, 'architecture'):
            verify_runtime(self.root)

    def test_damaged_elf_rejected(self):
        self.binary.write_bytes(b'\x7fELF')
        with self.assertRaisesRegex(ValueError, 'ELF'):
            verify_runtime(self.root)

    def test_source_mismatch_rejected(self):
        (self.root / 'source.json').write_text('{}')
        with self.assertRaisesRegex(ValueError, 'source'):
            verify_runtime(self.root)

    def test_missing_retry_evidence_rejected(self):
        (self.root / 'retry-fix.diff').unlink()
        with self.assertRaises((ValueError, FileNotFoundError)):
            verify_runtime(self.root)

    def test_valid_receipts_accepted(self):
        self.assertEqual(verify_runtime(self.root)['source_sha256'], runtime.SOURCE_SHA256)

    def test_altered_binary_rejected(self):
        with self.binary.open('ab') as stream:
            stream.write(b'changed')
        with self.assertRaisesRegex(ValueError, 'digest'):
            verify_runtime(self.root)

    def test_missing_retry_logic_rejected(self):
        (self.root / 'retry-fix.diff').write_text('unrelated')
        with self.assertRaisesRegex(ValueError, 'retry'):
            verify_runtime(self.root)

    def test_failed_build_rejected(self):
        receipt = self.root / 'build-evidence.json'
        build = json.loads(receipt.read_text())
        build['commands'][0]['exitCode'] = 1
        receipt.write_text(json.dumps(build))
        with self.assertRaisesRegex(ValueError, 'build'):
            verify_runtime(self.root)

    def test_source_digest_covers_rust_and_lock_files(self):
        source = self.root / 'source'
        (source / 'a.rs').write_text('fn main() {}')
        (source / 'Cargo.lock').write_text('version = 4')
        first = runtime.source_digest(source)
        (source / 'ignored.log').write_text('unrelated')
        self.assertEqual(runtime.source_digest(source), first)
        (source / 'Cargo.lock').write_text('version = 3')
        self.assertNotEqual(runtime.source_digest(source), first)

    def test_altered_source_snapshot_rejected(self):
        (self.root / 'source/added.rs').write_text('fn changed() {}')
        with self.assertRaisesRegex(ValueError, 'source'):
            verify_runtime(self.root)


class BudgetRuntimeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'runtime').mkdir()
        (self.root / 'source').mkdir()
        header = b'\x7fELF' + bytes([2, 1, 1]) + bytes(9) + struct.pack('<HHI', 2, 62, 1) + bytes(40)
        for name in ('shiju', 'vw-budget-gate'):
            (self.root / 'runtime' / name).write_bytes(header)
        for name in ('budget-gate.rs', 'budget-fix.diff', 'retry-fix.diff'):
            (self.root / name).write_text('fixture ' + name)
        self.expected = {'schema_version': 2, 'base_commit': runtime.BASE_COMMIT,
            'source_sha256': runtime.source_digest(self.root / 'source'),
            'parent_binary_sha256': 'a' * 64,
            'budget_gate_source_sha256': runtime.sha256(self.root / 'budget-gate.rs'),
            'patches': {name: runtime.sha256(self.root / name) for name in ('budget-fix.diff', 'retry-fix.diff')},
            'runtime_files': {p.name: runtime.sha256(p) for p in (self.root / 'runtime').iterdir()}}
        self.write('source.json', self.expected)
        self.receipt = {'sourceBefore': self.expected['source_sha256'], 'sourceAfter': self.expected['source_sha256'],
            'sourceBaseCommit': runtime.BASE_COMMIT, 'parentBinarySha256': 'a' * 64,
            'binarySha256': runtime.sha256(self.root / 'runtime/shiju'),
            'budgetGateSha256': runtime.sha256(self.root / 'runtime/vw-budget-gate'),
            'budgetGateSourceSha256': self.expected['budget_gate_source_sha256'],
            'commands': [{'command': ['cargo', 'build', '-p', 'vw-cli', '--bin', 'shiju', '--target',
                                      'x86_64-unknown-linux-gnu', '--locked'], 'exitCode': 0}]}
        self.write('build-evidence.json', self.receipt)
        expected = patch('suite.runtime.BUDGET_IDENTITY', self.expected)
        expected.start()
        self.addCleanup(expected.stop)

    def write(self, name, payload):
        (self.root / name).write_text(json.dumps(payload))

    def test_new_generation_records_parent_without_reusing_identity(self):
        result = verify_runtime(self.root)
        self.assertEqual(result['generation'], 2)
        self.assertEqual(result['parent_binary_sha256'], 'a' * 64)
        self.assertIn('vw-budget-gate', result['sidecars'])

    def test_modified_gate_and_unrecorded_sidecars_fail_closed(self):
        for name in ('vw-budget-gate', 'unexpected'):
            with self.subTest(name=name):
                path = self.root / 'runtime' / name
                before = path.read_bytes() if path.exists() else None
                path.write_bytes(b'drift')
                with self.assertRaises(ValueError):
                    verify_runtime(self.root)
                if before is None:
                    path.unlink()
                else:
                    path.write_bytes(before)

    def test_failed_build_receipt_or_changed_source_cannot_be_frozen(self):
        self.receipt['commands'][0]['exitCode'] = 1
        self.write('build-evidence.json', self.receipt)
        with self.assertRaises(ValueError):
            verify_runtime(self.root)
        self.receipt['commands'][0]['exitCode'] = 0
        self.write('build-evidence.json', self.receipt)
        (self.root / 'source/changed.rs').write_text('drift')
        with self.assertRaises(ValueError):
            verify_runtime(self.root)
