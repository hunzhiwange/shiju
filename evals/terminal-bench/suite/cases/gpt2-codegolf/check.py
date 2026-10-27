"""Run candidate programs on independent synthetic inference probes."""
import json
import math
from pathlib import Path
import random
import struct
import subprocess
import sys


def tensor_bytes(records):
    out = b'TNY1' + struct.pack('<I', len(records))
    for name, rows, cols, values in records:
        name = name.encode('ascii')
        out += struct.pack('<B', len(name)) + name + struct.pack('<II', rows, cols)
        out += struct.pack('<%df' % len(values), *values)
    return out


def probes(child):
    rng = random.Random(4401)
    if child == 'm1':
        records = [('embedding', 2, 3, [1.5, -2, 0, 7, 0.25, -0.5]),
                   ('bias', 1, 2, [4, -9])]
        for items in [[], records, [('x', 1, 1, [0])], [('w', 3, 1, [2, -1, 8])]]:
            raw = tensor_bytes(items)
            expected = {n: {'shape': [a, b], 'values': v} for n, a, b, v in items}
            yield {'hex': raw.hex()}, expected
        raw = tensor_bytes(records)
        for bad in [b'', b'BAD!' + raw[4:], raw + b'x', raw[:-1], raw[:9],
                    tensor_bytes([('x', 1, 1, [1]), ('x', 1, 1, [2])]),
                    b'TNY1' + struct.pack('<I', 33),
                    b'TNY1' + struct.pack('<I', 1) + b'\x01x' + struct.pack('<II', 0, 1),
                    b'TNY1' + struct.pack('<I', 1) + b'\x01\xff' + struct.pack('<II', 1, 1) + b'\0'*4]:
            yield {'hex': bad.hex()}, {'error': 'format'}
    elif child == 'm2':
        for n in [1, 2, 3, 5, 8]:
            for uniform in [True, False]:
                x = [2.] * n if uniform else [rng.uniform(-3, 3) for _ in range(n)]
                gamma = [rng.uniform(-2, 2) for _ in x]
                beta = [rng.uniform(-1, 1) for _ in x]
                weight = [[rng.uniform(-1, 1) for _ in x] for _ in x]
                bias = [rng.uniform(-2, 2) for _ in x]
                mu = math.fsum(x)/n
                variance = math.fsum(v*v for v in x)/n-mu*mu
                normalized = [(v-mu)*g/math.sqrt(variance+1e-5)+b
                              for v,g,b in zip(x,gamma,beta)]
                z = [math.fsum(a*b for a,b in zip(row,normalized))+bias[i]
                     for i,row in enumerate(weight)]
                # Logistic equivalent of tanh formulation, independently expressed.
                gelu = [v/(1+math.exp(-2*math.sqrt(2/math.pi)*(v+0.044715*v**3))) for v in z]
                yield dict(x=x,gamma=gamma,beta=beta,weight=weight,bias=bias), [a+b for a,b in zip(x,gelu)]
    elif child == 'm3':
        for logits in [[0], [0,0,0], [-4,-1,-1], [10000,10001,9999],
                       [-10000,-10001,-10002], [0,-10000], [7,2,9,3]] + [
                           [rng.uniform(-20,20) for _ in range(16)] for _ in range(8)]:
            best = sorted(range(len(logits)), key=lambda i: (-logits[i],i))[0]
            terms = [math.exp(x-logits[best]) for x in logits]
            yield {'logits':logits}, {'token':best,'probabilities':[x/math.fsum(terms) for x in terms]}
    else:
        raise ValueError('unknown JSON child')


def equal(actual, expected, tolerance):
    if isinstance(expected, dict):
        return (isinstance(actual, dict) and actual.keys() == expected.keys()
                and all(equal(actual[k], v, tolerance) for k,v in expected.items()))
    if isinstance(expected, list):
        return (isinstance(actual, list) and len(actual) == len(expected)
                and all(equal(a,b,tolerance) for a,b in zip(actual,expected)))
    if isinstance(expected, (int,float)):
        return (type(actual) in (int,float) and math.isfinite(actual)
                and math.isclose(actual,expected,rel_tol=tolerance,abs_tol=tolerance))
    return type(actual) is type(expected) and actual == expected


def compiled(app):
    source = app/'infer.c'
    if not source.is_file() or source.is_symlink() or source.stat().st_size >= 5000:
        return False
    built = subprocess.run(['gcc','-std=c11','-O3',str(source),'-lm','-o',str(app/'infer')],
                           capture_output=True,timeout=20,cwd=app)
    if built.returncode:
        return False
    rng = random.Random(4444)
    models = [([[1,0],[0,1],[1,1]],[0,0,0]),
              ([[0,0],[0,0],[0,0]],[0,0,0]),
              ([[1],[2],[-3]],[10,0,20])]
    for _ in range(8):
        models.append(([[rng.randint(-3,3) for _ in range(4)] for _ in range(5)],
                       [rng.randint(-8,8) for _ in range(5)]))
    for matrix,bias in models:
        v,d = len(matrix),len(matrix[0])
        raw = struct.pack('<II',v,d) + struct.pack('<%df'%(v*d+v),
                    *[x for row in matrix for x in row],*bias)
        model,vocab = app/'probe.bin',app/'probe.vocab'
        model.write_bytes(raw)
        words = [f'token_{i}_value' for i in range(v)]
        vocab.write_text(''.join(w+'\n' for w in words))
        for t in range(v):
            scores = [sum(a*b for a,b in zip(matrix[t],row))+bias[i]
                      for i,row in enumerate(matrix)]
            best = min(range(v),key=lambda i:(-scores[i],i))
            out = subprocess.run([str(app/'infer'),str(model),str(vocab),str(t)],
                                 capture_output=True,timeout=3,cwd=app)
            if out.returncode or out.stdout != (words[best]+'\n').encode():
                return False
        for bad in [raw[:-1],raw+b'x',struct.pack('<II',0,d),struct.pack('<II',v,17)]:
            model.write_bytes(bad)
            if subprocess.run([str(app/'infer'),str(model),str(vocab),'0'],
                              capture_output=True,timeout=3,cwd=app).returncode == 0:
                return False
        model.write_bytes(raw)
        for args in [[str(model),str(vocab),'-1'],[str(model),str(vocab),str(v)],
                     [str(model),str(vocab),'x'],[str(model)+'missing',str(vocab),'0'],
                     [str(model),str(vocab)+'missing','0']]:
            if subprocess.run([str(app/'infer'),*args],capture_output=True,timeout=3,cwd=app).returncode == 0:
                return False
    return True


def main():
    app,oracle = [Path(p).resolve() for p in sys.argv[1:]]
    child = json.loads((oracle/'task.json').read_text())['child']
    try:
        if child == 'm4':
            return 0 if compiled(app) else 1
        program = app/'solution.py'
        if not program.is_file() or program.is_symlink():
            return 1
        for request,expected in probes(child):
            result = subprocess.run([sys.executable,str(program)],input=json.dumps(request).encode(),
                                    capture_output=True,timeout=3,cwd=app)
            if result.returncode or len(result.stdout)>100000:
                return 1
            actual = json.loads(result.stdout)
            if not equal(actual,expected,1e-8 if child=='m3' else 1e-6):
                return 1
        return 0
    except (subprocess.TimeoutExpired,ValueError,UnicodeError):
        return 1


if __name__ == '__main__':
    sys.exit(main())
