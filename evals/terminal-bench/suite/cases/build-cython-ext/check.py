"""Compile submitted sources and probe native APIs in a fresh interpreter."""
import json
from pathlib import Path
import subprocess
import sys

BUILD = '''from setuptools import setup, Extension
from Cython.Build import cythonize
import numpy as np
import sys
names = sys.argv.pop(1).split(',')
setup(name='diagnostic', ext_modules=cythonize(
    [Extension(n, [n+'.pyx'], include_dirs=[np.get_include()]) for n in names],
    compiler_directives={'language_level': 3}))
'''
PROBE = '''import importlib, importlib.machinery, math, sys
from pathlib import Path
import numpy as np
assert np.__version__ == '2.3.0'
names=sys.argv[1].split(',')
modules=[importlib.import_module(n) for n in names]
for m in modules:
    assert isinstance(m.__loader__, importlib.machinery.ExtensionFileLoader)
    assert Path(m.__file__).resolve().is_relative_to(Path.cwd())
kind=sys.argv[2]
if kind == 'm2':
    m=modules[0]
    for values in ([], [0], [3,-7,11], [2**40, -(2**39), 17], [-2**40, -9]):
        a=np.array(values,dtype=np.int64)
        assert type(m.total(a)) is int and m.total(a) == sum(values)
    a=np.array([13,7,-19,2,5,0],dtype=np.int64)[::2]
    assert m.total(a) == -1
    try:
        m.total(np.array([1.5],dtype=np.float64))
    except (ValueError,TypeError):
        pass
    else:
        raise AssertionError('float64 accepted by integer interface')
elif kind == 'm3':
    assert [m.identity() for m in modules] == [11,23,37]
elif kind == 'm4':
    m=modules[0]
    for a,b,c,d in [(2,3,5,7),(-2,0.5,3,-4),(0,2,3,0),(1.25,-2.5,0.5,4)]:
        expected=a*d-b*c
        assert math.isclose(m.cross(a,b,c,d),expected,abs_tol=1e-10)
        assert math.isclose(m.determinant(a,b,c,d),expected,abs_tol=1e-10)
    for values in ([],[2.5],[2,-3,4],[0.5,-2.25,3,7.5,-1]):
        expected=sum((i+1)*v*(-1)**i for i,v in enumerate(values))
        assert math.isclose(m.signed_sum(values),expected,abs_tol=1e-10)
else:
    raise RuntimeError('unknown diagnostic')
'''


def check(app, oracle):
    expected = json.loads((oracle / 'expected.json').read_text())
    kind = expected['id']
    if kind == 'm1':
        try:
            return json.loads((app / 'report.json').read_text()) == expected['report']
        except (OSError, ValueError):
            return False
    modules = expected['modules']
    if kind == 'm3':
        if not (app / 'build.py').is_file():
            return False
        command = [sys.executable, 'build.py', 'build_ext', '--inplace']
    else:
        if any(not (app / (name + '.pyx')).is_file() for name in modules):
            return False
        command = [sys.executable, '-c', BUILD, ','.join(modules), 'build_ext', '--inplace']
    try:
        built = subprocess.run(command, cwd=app, capture_output=True, timeout=75)
        if built.returncode:
            return False
        result = subprocess.run([sys.executable, '-c', PROBE, ','.join(modules), kind],
                                cwd=app, capture_output=True, timeout=15)
        return result.returncode == 0
    except subprocess.TimeoutExpired:
        return False


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()) else 1)
