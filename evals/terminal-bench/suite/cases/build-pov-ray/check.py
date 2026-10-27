"""Run candidate entry points and independently inspect their real artifacts."""
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile


def command(argv, directory):
    return subprocess.run(argv, cwd=directory, capture_output=True, timeout=25)


def render(directory, scene, output, width, height):
    result = command(['/opt/legacy/povray', '+I' + scene, '+O' + output,
                      '+W' + str(width), '+H' + str(height), '+FT', '-D', '-P', '-V'], directory)
    if result.returncode:
        raise RuntimeError('reference renderer failed: ' + (result.stdout + result.stderr).decode(errors='replace'))


def pixels(path):
    raw = path.read_bytes()
    if len(raw) < 18 or raw[1:3] != bytes([0, 2]) or raw[16] != 24 or raw[17] & 48 != 32:
        raise ValueError('unsupported reference image')
    width, height = struct.unpack_from('<HH', raw, 12)
    offset = 18 + raw[0]
    if len(raw) != offset + width * height * 3:
        raise ValueError('truncated reference image')
    return (width, height), raw[offset:]


def evaluate(app, oracle):
    subject = json.loads((oracle / 'contract.json').read_text())['id']
    if not (app / 'answer.py').is_file():
        return False
    with tempfile.TemporaryDirectory(dir=app) as temporary:
        work = Path(temporary)
        for path in list(app.iterdir()):
            if path == work:
                continue
            if path.is_dir():
                shutil.copytree(path, work / path.name)
            else:
                shutil.copyfile(path, work / path.name)
        inputs = {str(p.relative_to(work)): hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in work.rglob('*') if p.is_file() and p.name != 'answer.py'}
        run = lambda *args: command(['python3', 'answer.py', *map(str, args)], work)
        if subject == 'm1':
            if run().returncode:
                return False
            expected = json.loads((oracle / 'expected.json').read_text())
            output = work / 'output'
            actual = {str(p.relative_to(output)): hashlib.sha256(p.read_bytes()).hexdigest()
                      for p in output.rglob('*') if p.is_file()}
            if actual != expected:
                return False
        elif subject == 'm2':
            if run().returncode:
                return False
            binary = work / 'output/pov-mini'
            if not binary.is_file() or binary.read_bytes()[:4] != b'\x7fELF':
                return False
            for n in (0, 1, 7, 19, 67):
                result = command([str(binary), str(n)], work)
                if result.returncode or result.stdout != f'{(n*n+3*n+7)%251}\n'.encode():
                    return False
        elif subject == 'm3':
            for scene, width, height in [('red.pov', 16, 12), ('green.pov', 20, 14)]:
                render(work, scene, 'reference.tga', width, height)
                expected_pixels = pixels(work / 'reference.tga')
                target = work / 'candidate.tga'
                target.unlink(missing_ok=True)
                if run(scene, target, width, height).returncode or not target.is_file():
                    return False
                if pixels(target) != expected_pixels:
                    return False
        elif subject == 'm4':
            render(work, 'scene.pov', 'first.tga', 16, 12)
            render(work, 'scene.pov', 'other.tga', 20, 14)
            raw = (work / 'first.tga').read_bytes()
            # Distinguish pixel differences, harmless metadata, shape and truncation.
            changed = bytearray(raw)
            changed[18 + raw[0]] ^= 127
            metadata = bytearray(raw)
            metadata[0] = 3
            variants = [
                ('same', raw, True, 0, [16, 12]),
                ('changed', bytes(changed), False, 1, [16, 12]),
                ('metadata', bytes(metadata[:18]) + b'abc' + raw[18:], True, 0, [16, 12]),
                ('size', (work / 'other.tga').read_bytes(), False, None, [20, 14]),
            ]
            for name, data, same, count, size in variants:
                (work / 'second.tga').write_bytes(data)
                result = run('first.tga', 'second.tga')
                expected = {'same': same, 'changed_pixels': count,
                            'first_size': [16, 12], 'second_size': size}
                if result.returncode or json.loads(result.stdout) != expected:
                    return False
            (work / 'second.tga').write_bytes(raw[:-3])
            if run('first.tga', 'second.tga').returncode == 0:
                return False
        else:
            raise ValueError('unknown diagnostic')
        return all((work / name).is_file()
                   and hashlib.sha256((work / name).read_bytes()).hexdigest() == value
                   for name, value in inputs.items())


if __name__ == '__main__':
    try:
        accepted = evaluate(Path(sys.argv[1]), Path(sys.argv[2]))
    except (OSError, ValueError, subprocess.TimeoutExpired):
        accepted = False
    sys.exit(0 if accepted else 1)
