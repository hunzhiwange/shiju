"""Execute synthetic submissions; compile and time the native entry independently."""
import json
import math
import os
from pathlib import Path
import random
import shlex
import shutil
import subprocess
import sys
import sysconfig
import tempfile


def reference(p):
    w, s, r = p['weights'], p['matrix'], p['returns']
    temp = [sum(s[i][j]*w[j] for j in range(len(w))) for i in range(len(w))]
    return {'risk': math.sqrt(sum(w[i]*temp[i] for i in range(len(w)))),
            'return': sum(w[i]*r[i] for i in range(len(w)))}


def inputs(seed):
    rng = random.Random(seed)
    values = []
    for n in (0, 1, 2, 5, 12):
        w = [rng.uniform(-1, 2) for _ in range(n)]
        s = [[(4+i if i == j else .03125) for j in range(n)] for i in range(n)]
        values.append({'weights': w, 'matrix': s,
                       'returns': [rng.uniform(-.3, .4) for _ in range(n)]})
    return values


def same(actual, expected):
    return (isinstance(actual, dict) and set(actual) == set(expected)
            and all(type(actual[k]) in (float, int) and math.isfinite(actual[k])
                    and abs(actual[k]-v) < 1e-10 for k, v in expected.items()))


def run(command, work, data=None):
    return subprocess.run(command, cwd=work, input=data, capture_output=True,
                          timeout=25, env={'PATH': os.environ.get('PATH', os.defpath),
                                           'PYTHONDONTWRITEBYTECODE': '1'})


# Executed in a fresh interpreter after compilation. The checker owns timings,
# baseline, inputs and extension-origin checks; a submitted report is irrelevant.
NATIVE_PROBE = r'''
import importlib.machinery,json,math,sys,time
sys.path.insert(0,sys.argv[1])
import portfolio_native as m
assert isinstance(m.__loader__,importlib.machinery.ExtensionFileLoader)
assert isinstance(m.calculate,type(math.sqrt))
def baseline(w,s,r):
    n=len(w);q=0.;ret=0.
    for i in range(n):
        row=0.
        for j in range(n): row+=s[i*n+j]*w[j]
        q+=w[i]*row
        ret+=w[i]*r[i]
    return math.sqrt(q),ret
def timed(fn,*args):
    times=[]
    for _ in range(3):
        start=time.perf_counter(); result=fn(*args);times.append(time.perf_counter()-start)
    return min(times),result
for n in (1,3,17,320):
    w=[((i*19)%31-7)/37 for i in range(n)]
    r=[((i*11)%23-9)/71 for i in range(n)]
    s=[(3+i*.003 if i==j else .015625) for i in range(n) for j in range(n)]
    expected=baseline(w,s,r);actual=m.calculate(w,s,r)
    assert isinstance(actual,tuple) and len(actual)==2
    assert all(type(a) in (float,int) and math.isfinite(a) and abs(a-b)<1e-10
               for a,b in zip(actual,expected))
    if n==320:
        bt,_=timed(baseline,w,s,r);nt,_=timed(m.calculate,w,s,r)
        print(json.dumps({'baseline_seconds':bt,'native_seconds':nt,'speedup':bt/nt}))
        assert nt>0 and bt/nt>=1.2
'''


def native(app):
    source = app / 'native.c'
    if source.is_symlink() or not source.is_file() or source.stat().st_size > 65536:
        return False
    cc = shlex.split(sysconfig.get_config_var('LDSHARED') or '')
    include = Path(sysconfig.get_path('include'))
    if not cc or not shutil.which(cc[0]) or not (include / 'Python.h').is_file():
        raise RuntimeError('C compiler or CPython headers missing: infrastructure failure')
    with tempfile.TemporaryDirectory(prefix='.native-check-', dir=app) as directory:
        work = Path(directory)
        candidate = work / 'native.c'
        candidate.write_bytes(source.read_bytes())
        output = work / ('portfolio_native' + sysconfig.get_config_var('EXT_SUFFIX'))
        result = run([*cc, '-O3', '-fPIC', '-I'+str(include), str(candidate), '-o', str(output), '-lm'], work)
        if result.returncode:
            sys.stderr.buffer.write(result.stderr)
            return False
        probe = work / 'probe.py'
        probe.write_text(NATIVE_PROBE)
        result = run([sys.executable, '-I', str(probe), str(work)], work)
        sys.stdout.buffer.write(result.stdout)
        sys.stderr.buffer.write(result.stderr)
        return result.returncode == 0


def check(app, oracle):
    task = json.loads((oracle / 'task.json').read_text())
    kind = task['kind']
    if kind == 'm4':
        return native(app)
    if kind not in ('m1', 'm2', 'm3'):
        raise ValueError('unknown child')
    source = app / ('layout.py' if kind == 'm1' else 'kernel.py')
    if source.is_symlink() or not source.is_file() or source.stat().st_size > 65536:
        return False
    cases = []
    if kind == 'm1':
        rng = random.Random(task['seed'])
        for n in (1, 2, 4, 7):
            matrix = [[rng.randrange(-9, 10) for _ in range(n)] for _ in range(n)]
            for order in ('row', 'column'):
                flat = ([v for row in matrix for v in row] if order == 'row'
                        else [matrix[i][j] for j in range(n) for i in range(n)])
                p = {'weights': [i+.25 for i in range(n)], 'returns': [i-2.5 for i in range(n)],
                     'matrix': flat, 'order': order}
                cases.append((p, {'weights': p['weights'], 'matrix': matrix, 'returns': p['returns']}))
    else:
        cases = [(p, reference(p)) for p in inputs(task['seed'])]
        if kind == 'm3':
            cases += [(p, None) for p in (
                {'weights': [1, 2], 'matrix': [[4]], 'returns': [.1]},
                {'weights': [1, 2], 'matrix': [[4, 0], [0]], 'returns': [.1, .2]},
                {'weights': [1], 'matrix': [[-2]], 'returns': [.1]},
                {'weights': [1], 'matrix': [[2]], 'returns': [float('inf')]},
                {'weights': [float('nan')], 'matrix': [[2]], 'returns': [.1]})]
    for p, expected in cases:
        result = run([sys.executable, '-I', str(source)], app, json.dumps(p).encode())
        if expected is None:
            if result.returncode == 0 or result.stdout.strip():
                return False
            continue
        if result.returncode:
            return False
        try:
            actual = json.loads(result.stdout)
        except (ValueError, UnicodeError):
            return False
        if (actual != expected if kind == 'm1' else not same(actual, expected)):
            return False
    return True


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
    except (OSError, subprocess.TimeoutExpired):
        passed = False
    sys.exit(0 if passed else 1)
