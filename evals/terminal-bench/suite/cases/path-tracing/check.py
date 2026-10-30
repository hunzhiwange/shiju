"""Independent synthetic pixel checks; execute submissions on fresh unseen inputs."""
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import zlib


def ppm(text):
    words = ' '.join(line.split('#')[0] for line in text.splitlines()).split()
    if not words or words[0] != 'P3':
        raise ValueError('P3 required')
    width, height, maximum = map(int, words[1:4])
    pixels = list(map(int, words[4:]))
    if width < 1 or height < 1 or not 1 <= maximum <= 65535:
        raise ValueError('invalid header')
    if len(pixels) != width * height * 3 or any(not 0 <= c <= maximum for c in pixels):
        raise ValueError('invalid samples')
    return width, height, maximum, pixels


def run(command, work, value=None):
    result = subprocess.run(command, cwd=work, input=json.dumps(value) if value is not None else '',
                            text=True, capture_output=True, timeout=5,
                            env={'PATH': os.defpath, 'PYTHONDONTWRITEBYTECODE': '1'})
    if result.returncode or len(result.stdout) > 262144:
        raise ValueError('candidate failed')
    return result.stdout


def check(app, oracle):
    task = json.loads((oracle / 'task.json').read_text())
    kind = task['kind']
    name = {'m1': 'inspect.py', 'm2': 'render.py', 'm3': 'coordinates.py', 'm4': 'image.c'}[kind]
    source = app / name
    if source.is_symlink() or not source.is_file():
        return False
    data = source.read_bytes()
    if len(data) > 65536 or (kind == 'm4' and len(zlib.compress(data)) >= 2100):
        return False
    for value in task['inputs']:
        with tempfile.TemporaryDirectory(prefix='.path-tracing-check-', dir=app) as directory:
            work = Path(directory)
            candidate = work / name
            candidate.write_bytes(data)
            if kind == 'm4':
                compiler = shutil.which('gcc' if sys.platform.startswith('linux') else 'cc')
                if compiler is None:
                    raise RuntimeError('compiler unavailable')
                flags = ['-static'] if sys.platform.startswith('linux') else []
                run([compiler, *flags, '-o', str(work/'image'), str(candidate), '-lm'], work)
                # Execute only the compiled binary, without candidate source or fixture image.
                candidate.unlink()
                run([str(work/'image'), *map(str, value)], work)
                output = work / 'reconstructed.ppm'
                if output.is_symlink() or output.stat().st_size > 262144:
                    return False
                actual = ppm(output.read_text())
                width, height, seed = value
                pixels = []
                for row in range(height):
                    for col in range(width):
                        pixels.extend(((17*col+seed) % 256, (31*row+seed) % 256,
                                       (col+3*row+seed) % 256))
                expected = (width, height, 255, pixels)
            else:
                output = run([sys.executable, str(candidate)], work, value)
                if kind == 'm1':
                    width, height, _, pixels = ppm(value['ppm'])
                    expected = {'width': width, 'height': height, 'pixels': []}
                    for col, row in value['queries']:
                        offset = 3*(row*width+col)
                        expected['pixels'].append(pixels[offset:offset+3])
                    actual = json.loads(output)
                elif kind == 'm2':
                    width, height = value['width'], value['height']
                    pixels = []
                    for row in range(height):
                        for col in range(width):
                            inside = math.hypot(col-value['cx'], row-value['cy']) <= value['radius']
                            pixels.extend(value['color'] if inside else value['background'])
                    expected = (width, height, 255, pixels)
                    actual = ppm(output)
                else:
                    expected = [{'cell': [math.floor(x), math.floor(y)],
                                 'rgb': [math.floor(255.99*max(0, min(c, 1))) for c in rgb]}
                                for x, y, rgb in value]
                    actual = json.loads(output)
            if json.dumps(actual, sort_keys=True, allow_nan=False) != json.dumps(expected, sort_keys=True):
                return False
    return True


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]), Path(sys.argv[2]))
    except (OSError, ValueError, TypeError, KeyError, subprocess.TimeoutExpired):
        passed = False
    sys.exit(0 if passed else 1)
