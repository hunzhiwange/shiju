"""Verify the historical Linux runtime without attributing it to current HEAD."""
import hashlib
import json
from pathlib import Path
import struct

BINARY_SHA256 = '7ad73fbb2031953dc1fcdca03f16c5cc95229bb02968d17f331f70f066ba75de'
SOURCE_SHA256 = 'ad57e0be7290755ca2885ecd7a1e1ee35cbbf603f9ae0c1ae78042ccb6ca788b'
BASE_COMMIT = '470278872a22f3cc30a5ed18bd89c55ebf403e36'
BUDGET_IDENTITY = json.loads(Path(__file__).with_name('runtime-v2.json').read_text())


def sha256(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def source_digest(root):
    digest = hashlib.sha256()
    for path in sorted((p for p in root.rglob('*') if p.is_file() and
                        (p.suffix == '.rs' or p.name in ('Cargo.toml', 'Cargo.lock'))),
                       key=lambda p: str(p.relative_to(root))):
        digest.update(str(path.relative_to(root)).encode() + b'\0' + path.read_bytes() + b'\0')
    return digest.hexdigest()


def verify_runtime(root):
    """Reject missing, altered or incompatible historical artifacts; never execute ELF."""
    root = Path(root)
    if json.loads((root / 'source.json').read_text()).get('schema_version') == 2:
        return verify_budget_runtime(root)
    binary = root / 'runtime/shiju'
    with binary.open('rb') as stream:
        header = stream.read(64)
    if len(header) != 64 or header[:7] != b'\x7fELF\x02\x01\x01':
        raise ValueError('invalid ELF header')
    if struct.unpack_from('<H', header, 18)[0] != 62:
        raise ValueError('wrong ELF architecture: expected x86_64')
    source = json.loads((root / 'source.json').read_text())
    if (source.get('baseCommit') != BASE_COMMIT or
            source.get('fixedSourceSha256') != SOURCE_SHA256 or
            source_digest(root / 'source') != SOURCE_SHA256):
        raise ValueError('frozen source identity mismatch')
    patch = root / 'retry-fix.diff'
    build = json.loads((root / 'build-evidence.json').read_text())
    if (source.get('onlyProductionPatch') != patch.name or
            patch.read_bytes() != (root / 'runtime-source.diff').read_bytes() or
            b'+        Ok(AssistantError::APIError { is_retryable: false, .. })' not in patch.read_bytes()):
        raise ValueError('missing retry repair evidence')
    binary_hash = sha256(binary)
    if binary_hash != BINARY_SHA256 or build.get('binarySha256') != binary_hash:
        raise ValueError('frozen ELF digest mismatch')
    if (build.get('sourceBefore') != SOURCE_SHA256 or build.get('sourceAfter') != SOURCE_SHA256 or
            build.get('sourceCommit') != BASE_COMMIT or
            build.get('CFLAGS_x86_64_unknown_linux_gnu') != '-DSHA1DC_FORCE_ALIGNED_ACCESS=1' or
            not any(c.get('exitCode') == 0 and c.get('command') ==
                    ['cargo', 'build', '-p', 'vw-cli', '--bin', 'shiju', '--target',
                     'x86_64-unknown-linux-gnu', '--locked'] for c in build.get('commands', []))):
        raise ValueError('frozen build receipt mismatch')
    return {'binary_sha256': binary_hash, 'source_sha256': SOURCE_SHA256,
            'source_base_commit': BASE_COMMIT, 'retry_patch_sha256': sha256(patch),
            'sidecars': {p.name: sha256(p) for p in sorted((root / 'runtime').iterdir())
                         if p.is_file() and p.name != 'shiju'},
            'identity_scope': 'historical frozen ELF; current source tests are separate'}


def verify_budget_runtime(root):
    """Verify the explicitly rebuilt generation without accepting self-declared IDs."""
    expected = BUDGET_IDENTITY
    if json.loads((root / 'source.json').read_text()) != expected:
        raise ValueError('budget runtime identity mismatch')
    if source_digest(root / 'source') != expected['source_sha256']:
        raise ValueError('budget runtime source drift')
    if sha256(root / 'budget-gate.rs') != expected['budget_gate_source_sha256']:
        raise ValueError('budget gate source drift')
    for name, value in expected['patches'].items():
        if sha256(root / name) != value:
            raise ValueError('budget runtime patch drift')
    files = {p.name: sha256(p) for p in (root / 'runtime').iterdir() if p.is_file()}
    if files != expected['runtime_files']:
        raise ValueError('budget runtime binary or sidecar drift')
    for name in ('shiju', 'vw-budget-gate'):
        with (root / 'runtime' / name).open('rb') as stream:
            header = stream.read(64)
        if (len(header) != 64 or header[:7] != b'\x7fELF\x02\x01\x01'
                or struct.unpack_from('<H', header, 18)[0] != 62):
            raise ValueError('budget runtime requires x86_64 ELF')
    receipt = json.loads((root / 'build-evidence.json').read_text())
    if (receipt.get('sourceBefore') != expected['source_sha256']
            or receipt.get('sourceAfter') != expected['source_sha256']
            or receipt.get('sourceBaseCommit') != expected['base_commit']
            or receipt.get('parentBinarySha256') != expected['parent_binary_sha256']
            or receipt.get('binarySha256') != files['shiju']
            or receipt.get('budgetGateSha256') != files['vw-budget-gate']
            or receipt.get('budgetGateSourceSha256') != expected['budget_gate_source_sha256']
            or not any(c.get('exitCode') == 0 and c.get('command') ==
                       ['cargo', 'build', '-p', 'vw-cli', '--bin', 'shiju', '--target',
                        'x86_64-unknown-linux-gnu', '--locked'] for c in receipt.get('commands', []))):
        raise ValueError('budget runtime build evidence mismatch')
    return {'generation': 2, 'binary_sha256': files['shiju'],
            'source_sha256': expected['source_sha256'], 'source_base_commit': expected['base_commit'],
            'parent_binary_sha256': expected['parent_binary_sha256'],
            'retry_patch_sha256': expected['patches']['retry-fix.diff'],
            'budget_patch_sha256': expected['patches']['budget-fix.diff'],
            'sidecars': {k: v for k, v in files.items() if k != 'shiju'},
            'identity_scope': 'historical source plus budget repair; current source tests are separate'}


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('historical_root', type=Path)
    args = parser.parse_args()
    print(json.dumps(verify_runtime(args.historical_root), indent=2, sort_keys=True))
