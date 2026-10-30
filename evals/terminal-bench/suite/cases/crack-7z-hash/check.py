"""Execute submitted archive programs against real synthetic 7z files."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def snapshot(root):
    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob('*') if p.is_file()}


def check(app, oracle):
    spec = json.loads((oracle / 'spec.json').read_text())
    program = app / spec['program']
    if not program.is_file() or program.is_symlink():
        return False
    before = snapshot(app)
    for hidden in (False, True):
        with tempfile.TemporaryDirectory() as temp:
            work = Path(temp)
            archive = work / 'input archive.7z'
            shutil.copyfile(oracle / 'probe.7z' if hidden else app / 'sample.7z', archive)
            archive_before = archive.read_bytes()
            password = 'probe-58' if hidden and spec['child'] != 'm1' else spec['password']

            def run(*args):
                return subprocess.run([sys.executable, '-I', str(program), *map(str, args)],
                                      cwd=work, capture_output=True, timeout=12)

            if spec['child'] == 'm1':
                result = run(archive)
                expected = [{'path': 'private/target note.txt', 'size': 6},
                            {'path': 'private/empty.bin', 'size': 0},
                            {'path': 'readme.txt', 'size': 18}]
                if hidden:
                    expected.append({'path': 'extra dir/other.txt', 'size': 13})
                value = json.loads(result.stdout)
                if (result.returncode or value != sorted(expected, key=lambda x: x['path'])
                        or any(type(v['size']) is not int for v in value)):
                    return False
            elif spec['child'] == 'm2':
                dictionary = work / 'candidates.txt'
                dictionary.write_text('incorrect\n' + password + '\nlast-wrong\n')
                result = run(archive, dictionary)
                if result.returncode or json.loads(result.stdout) != {'password': password}:
                    return False
                dictionary.write_text('not-the-password\nalso-wrong\n')
                if run(archive, dictionary).returncode == 0:
                    return False
                if dictionary.read_text() != 'not-the-password\nalso-wrong\n':
                    return False
            else:
                target = 'private/target note.txt' if spec['child'] == 'm3' else 'nested/secret_file.txt'
                expected = b'amber\n' if spec['child'] == 'm3' else '海风 α\n\n'.encode()
                if hidden:
                    expected = b'cobalt\n' if spec['child'] == 'm3' else '静夜 β\n'.encode()
                output = work / 'saved result.txt'
                result = run(archive, password, target, output)
                if result.returncode or not output.is_file() or output.read_bytes() != expected:
                    return False
                output.unlink()
                if spec['child'] == 'm3':
                    for invalid_password, member in [('wrong-password', target), (password, 'absent.txt')]:
                        result = run(archive, invalid_password, member, output)
                        if result.returncode == 0 or output.exists():
                            return False
                    result = run(archive, password, 'readme.txt', output)
                    if result.returncode or output.read_bytes() != b'synthetic fixture\n':
                        return False
                    output.unlink()
            if archive.read_bytes() != archive_before:
                return False
            allowed = {'input archive.7z', 'candidates.txt'} if spec['child'] == 'm2' else {'input archive.7z'}
            if {str(p.relative_to(work)) for p in work.rglob('*')} - allowed:
                return False
    return snapshot(app) == before


if __name__ == '__main__':
    try:
        accepted = check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError):
        accepted = False
    sys.exit(0 if accepted else 1)
