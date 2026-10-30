"""Execute candidate programs on fresh synthetic working trees, never self-reports."""
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile

KINDS = {'AWS_ID': '<your-aws-access-key-id>',
         'AWS_SECRET': '<your-aws-secret-access-key>',
         'GITHUB': '<your-github-token>', 'HF': '<your-huggingface-token>'}
GIT_ENV = {**os.environ, 'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': '/dev/null'}


def snapshot(root):
    result = {}
    for p in root.rglob('*'):
        name = p.relative_to(root).as_posix()
        if p.is_symlink():
            result[name] = ('link', os.readlink(p))
        elif p.is_file():
            result[name] = (stat.S_IMODE(p.stat().st_mode), p.read_bytes())
        elif p.is_dir():
            result[name] = ('directory',)
    return result


def setup(root, index):
    root.mkdir()
    tokens = {kind: f'SYNTH_{kind}_FICT{index:04d}' for kind in KINDS}
    dirty = {
        f'config/worker-{index}.env': (
            '# preserve this comment\r\n' + ''.join(f'{kind}="{value}"\r\n' for kind, value in tokens.items())
            + f'repeated={tokens["GITHUB"]} {tokens["GITHUB"]}\r\n'),
        f'.local/cache-{index}.txt': f'nested={tokens["HF"]} {tokens["AWS_ID"]}\n',
        f'src/job-{index}.py': f'COUNT = 7\nTOKEN = "{tokens["AWS_SECRET"]}"\n',
    }
    normal = {'README.md': b'Keep this documentation.\n',
              'assets/payload.bin': b'\xff\x00\xfe\x10',
              'config/clean.json': b'{ "workers": 7, "region": "demo" }\n',
              'run.sh': b'#!/bin/sh\nprintf "ok\\n"\n',
              'near.txt': (' '.join(f'SYNTH_{k}_SHORT SYNTH_{k}_ABCDEFGHI x{v} {v}x <ordinary>'
                                    for k, v in tokens.items()) + '\n'
                           + ' '.join(KINDS.values()) + '\n').encode()}
    clean = {}
    for name, value in dirty.items():
        result = value
        for kind, token in tokens.items():
            result = result.replace(token, KINDS[kind])
        clean[name] = result.encode()
    for name, content in {**normal, **{n: v.encode() for n, v in dirty.items()}}.items():
        p = root / name
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(content)
    (root / 'run.sh').chmod(0o755)
    # Real Git metadata must remain untouched, including objects containing tokens.
    def git(*args):
        subprocess.run(['git', '-C', str(root), *args], env=GIT_ENV,
                       check=True, capture_output=True, timeout=10)
    git('init', '-b', 'main')
    git('config', 'user.name', 'Synthetic Fixture')
    git('config', 'user.email', 'fixture@example.invalid')
    git('add', '.')
    git('commit', '-m', 'Synthetic input')
    occurrences = []
    for name, value in dirty.items():
        for line_no, line in enumerate(value.splitlines(), 1):
            for kind, token in tokens.items():
                occurrences += [dict(path=name, line=line_no, kind=kind)] * line.count(token)
    occurrences.sort(key=lambda x: (x['path'], x['line'], x['kind']))
    hashes = {n: hashlib.sha256(v).hexdigest() for n, v in {**normal, **clean}.items()}
    return dirty, clean, normal, occurrences, dict(hashes=hashes, normal=list(normal), sanitized=list(clean))


def check(app, oracle):
    child = json.loads((oracle / 'task.json').read_text())['child']
    program = app / 'solution.py'
    if not program.is_file() or program.is_symlink():
        return 1
    variants = ['clean', 'leaked', 'wrong-placeholder', 'normal-changed', 'missing', 'extra'] if child == 'm4' else range(3)
    with tempfile.TemporaryDirectory(prefix='sanitize-fixture-') as directory:
        scratch = Path(directory)
        for index, variant in enumerate(variants):
            root = scratch / f'repo-{index}'
            dirty, clean, normal, occurrences, manifest = setup(root, index)
            target = root / f'config/worker-{index}.env'
            args = [str(target if child == 'm2' else root)]
            expected_scan = occurrences
            if child == 'm4':
                for name, content in clean.items():
                    (root / name).write_bytes(content)
                if variant == 'leaked':
                    name = f'.local/cache-{index}.txt'
                    (root / name).write_bytes(dirty[name].encode())
                    expected_scan = [x for x in occurrences if x['path'] == name]
                else:
                    expected_scan = []
                if variant == 'wrong-placeholder':
                    target.write_bytes(clean[target.relative_to(root).as_posix()].replace(b'<your-github-token>', b'<redacted>'))
                elif variant == 'normal-changed':
                    (root / 'README.md').write_text('corrupted\n')
                elif variant == 'missing':
                    target.unlink()
                elif variant == 'extra':
                    (root / 'extra.txt').write_text('unexpected\n')
                manifest_path = scratch / f'manifest-{index}.json'
                manifest_path.write_text(json.dumps(manifest))
                args += [str(manifest_path)]
                expected = dict(leaks=expected_scan,
                                preserved=variant not in ('normal-changed', 'missing', 'extra'),
                                placeholders_ok=variant not in ('leaked', 'wrong-placeholder', 'missing'),
                                ready=variant == 'clean')
            before = snapshot(scratch)
            expected_tree = before.copy()
            if child in ('m2', 'm3'):
                replacements = clean if child == 'm3' else {f'config/worker-{index}.env': clean[f'config/worker-{index}.env']}
                for name, content in replacements.items():
                    key = f'repo-{index}/{name}'
                    expected_tree[key] = (before[key][0], content)
            try:
                result = subprocess.run([sys.executable, '-B', str(program), *args],
                                        capture_output=True, timeout=15)
            except subprocess.TimeoutExpired:
                return 1
            if result.returncode:
                return 1
            if child in ('m1', 'm4'):
                try:
                    actual = json.loads(result.stdout)
                except (ValueError, UnicodeError):
                    return 1
                expected = occurrences if child == 'm1' else expected
                # JSON serialization distinguishes booleans from integer impostors.
                if json.dumps(actual, sort_keys=True) != json.dumps(expected, sort_keys=True):
                    return 1
            if snapshot(scratch) != expected_tree:
                return 1
    return 0


if __name__ == '__main__':
    sys.exit(check(*map(Path, sys.argv[1:])))
