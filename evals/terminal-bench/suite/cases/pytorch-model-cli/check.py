"""Run candidate programs on independently specified synthetic controls."""
import json
import math
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import zlib


def png(width, height, pixels):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    raw = b''.join(b'\0' + bytes(pixels[i*width:(i+1)*width]) for i in range(height))
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 0, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b''))


def close(actual, expected):
    if isinstance(expected, dict):
        return isinstance(actual, dict) and actual.keys() == expected.keys() and all(close(actual[k], v) for k, v in expected.items())
    if isinstance(expected, list):
        return isinstance(actual, list) and len(actual) == len(expected) and all(close(a, b) for a, b in zip(actual, expected))
    return type(actual) in (int, float) and math.isfinite(actual) and math.isclose(actual, expected, rel_tol=1e-8, abs_tol=1e-8)


def check(app, oracle):
    child = json.loads((oracle / 'task.json').read_text())['child']
    program = app / 'solution.py'
    if not program.is_file():
        return False
    with tempfile.TemporaryDirectory(prefix='model-cli-control-') as directory:
        work = Path(directory)
        def data(name, value):
            (work / name).write_text(json.dumps(value))
        def run(args, expected=None, error=False, exact=None):
            try:
                result = subprocess.run([sys.executable, str(program), *args], cwd=work, capture_output=True, timeout=5)
            except subprocess.TimeoutExpired:
                return False
            if error:
                return result.returncode == 2 and result.stdout == b'' and bool(result.stderr)
            if result.returncode != 0 or result.stderr:
                return False
            if exact is not None:
                return result.stdout == exact and (work / 'prediction.txt').is_file() and (work / 'prediction.txt').read_bytes() == exact
            try:
                return close(json.loads(result.stdout), expected)
            except (ValueError, UnicodeError):
                return False
        weights, image = str(work / 'weights.json'), str(work / 'image.png')
        if child == 'm1':
            for values in ([0,17,85,255], [255,128,64,0]):
                data('weights.json', {'w1':[1,2,3,4,5,6], 'b1':[0,1], 'w2':[2,3,4], 'b2':[0]})
                (work / 'image.png').write_bytes(png(2,2,values))
                if not run([weights,image], {'shape':[2,2], 'pixels':[v/255 for v in values], 'counts':{'w1':6,'b1':2,'w2':3,'b2':1}}):
                    return False
        elif child == 'm2':
            data('weights.json', {'w1':[[1,-2,0],[0,1,1]],'b1':[-1,2],'w2':[[1,-1],[-2,0],[0,1]],'b2':[1,-3,2]})
            for vector, expected in [([2,1,-4],[1,-3,2]), ([5,0,1],[2,-11,5]), ([-1,2,3],[-6,-3,9])]:
                data('vector.json', vector)
                if not run([weights,str(work / 'vector.json')], expected):
                    return False
            data('weights.json', {'w1':[[0,1,0],[1,0,0]],'b1':[0,0],'w2':[[1,0],[0,1],[-1,1]],'b2':[0,1,-2]})
            data('vector.json', [2,3,0])
            if not run([weights,str(work / 'vector.json')],[3,3,-3]):
                return False
        elif child == 'm3':
            data('weights.json', {'weight':[1,-2,3,-4],'bias':0.75})
            (work / 'image.png').write_bytes(png(2,2,[255,0,85,0]))
            if not run([weights,image],2.75):
                return False
            (work / 'image.png').write_bytes(png(2,2,[0,255,0,255]))
            if not run([weights,image],-5.25):
                return False
            for args in ([],[weights],[weights,image,'extra'],['absent.json',image],[weights,'absent.png']):
                if not run(args,error=True):
                    return False
            for content in (b'not png',png(1,2,[1,2])):
                (work / 'image.png').write_bytes(content)
                if not run([weights,image],error=True):
                    return False
            (work / 'image.png').write_bytes(png(2,2,[0,1,2,3]))
            for value in ({}, {'weight':[1],'bias':0}, {'weight':['x',0,0,0],'bias':0}):
                data('weights.json', value)
                if not run([weights,image],error=True):
                    return False
            (work / 'weights.json').write_text('{broken')
            if not run([weights,image],error=True):
                return False
        elif child == 'm4':
            for logits, expected in [([0,0,0,0,0,0,0,0,0,1],b'9\n'), ([3,3,0,0,0,0,0,0,0,0],b'0\n'), ([-5,-6,-7,-8,-9,-10,-11,-12,-1,-2],b'8\n')]:
                data('logits.json',logits)
                (work / 'prediction.txt').write_text('stale output\n')
                if not run([str(work / 'logits.json')],exact=expected):
                    return False
        else:
            raise ValueError('unknown child')
    return True


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()) else 1)
