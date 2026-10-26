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
