"""Probe actual submitted programs; hidden APIs differ from public examples."""
import json
import math
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def network(weights, biases, outgoing, bias):
    return dict(weights=weights, biases=biases, output_weights=outgoing, output_bias=bias)


def probes(child):
    """Return independent input/expected/API triples, never execute gold code."""
    if child == 'm1':
        for source, queries, expected in [
            ('return -2.5', [-4, 0, 3, 3], [-2.5]*4),
            ('return max(0, x-0.5)-2*max(0, -x-1)+0.25',
             [-3, -1, 0, 0.5, 2, 0.5], [-3.75, 0.25, 0.25, 0.25, 1.75, 0.25]),
            ('return -3*max(0, 2*x+1)+max(0, x-2)-1',
             [-2, -0.5, 0, 2, 3], [-1, -1, -4, -16, -21]),
            ('return 0.125*x-0.25', [], [])]:
            yield {'queries': queries}, expected, 'def forward(x):\n    '+source+'\n'
    elif child == 'm2':
        # Integrate prescribed interval slopes to construct a fresh API.
        # Expected knot identities are explicit, independent of the candidate.
        for grid, slopes, knots in [
            ([-3,-2,-1,0,1,2,3], [0,0,2,2,-1,4], [-1,1,2]),
            ([-4,-1,0,2,7], [3,-2,-2,1], [-1,2]),
            ([-2,-1,0,1], [-0.5]*3, []),
            ([0,0.1,0.4,2], [2,-3,1], [0.1,0.4]),
            ([-10,0,10], [0.01,0], [0])]:
            values=[1.25]
            for i,slope in enumerate(slopes):
                values.append(values[-1]+slope*(grid[i+1]-grid[i]))
            source=('G='+repr(grid)+'\nY='+repr(values)+'\nS='+repr(slopes)+'\n'
                    'def forward(x):\n'
                    '    for i in range(len(S)):\n'
                    '        if x <= G[i+1]:\n'
                    '            return Y[i]+S[i]*(x-G[i])\n'
                    '    return Y[-1]+S[-1]*(x-G[-1])\n')
            yield {'grid':grid}, knots, source
    elif child == 'm3':
        for original, expected in [
            (network([[3,4],[-2,0]],[5,2],[2,-3],1),
             network([[-1,0],[0.6,0.8]],[1,1],[-6,10],1)),
            (network([[0,-3],[0,-6],[2,0]],[-6,3,0],[4,-1,0],-2),
             network([[0,-1],[0,-1],[1,0]],[-2,0.5,0],[12,-6,0],-2)),
            (network([[-0.5],[0.25],[-2]],[1,-0.5,4],[2,8,0.5],0),
             network([[-1],[-1],[1]],[2,2,-2],[1,1,2],0)),
            (network([[1,2,2]],[6],[-4],0.125),
             network([[1/3,2/3,2/3]],[2],[-12],0.125)),
            (network([[2,0],[2,0],[2,0]],[2,0,0],[1,3,-1],-1),
             network([[1,0],[1,0],[1,0]],[0,0,1],[-2,6,2],-1))]:
            yield original, expected, None
    elif child == 'm4':
        original=network([[1,0],[0,-2]],[-1,1],[2,-1],0.25)
        points=[[0,0],[2,0],[-1,2],[1,-1],[3,3]]
        expected=[-0.75,1.25,0.25,-2.75,4.25]
        for candidate, outputs, tolerance in [
            (network([[0,-4],[3,0]],[2,-3],[-0.5,2/3],0.25),expected,0),
            (network([[1,0],[0,-2]],[-1,1],[2,-1],0.375),
             [x+0.125 for x in expected],0.125),
            (network([[1,0],[0,-2]],[-1,1],[2,-1],0.375),
             [x+0.125 for x in expected],0.124),
            (network([[1,0]],[0],[0],0),[0]*5,1e-8),
            (network([[-1,0],[0,-2]],[1,1],[2,-1],0.25),
             [1.25,-0.75,4.25,-2.75,0.25],1e-8)]:
            differences=[a-b for a,b in zip(outputs,expected)]
            yield {'reference':original,'candidate':candidate,'inputs':points,
                   'tolerance':tolerance}, {
                       'differences':differences,'max_error':max(map(abs,differences)),
                       'equivalent':all(abs(x)<=tolerance for x in differences)}, None
        # Width and dimension differ from the public sample and earlier probes.
        yield {'reference':network([[1]],[0],[1],-2),
               'candidate':network([[2],[3]],[0,0],[0.25,1/6],-2),
               'inputs':[[-1],[0],[4]],'tolerance':1e-9}, {
                   'differences':[0,0,0],'max_error':0,'equivalent':True}, None
    else:
        raise ValueError('unknown child')


def equivalent(actual, expected):
    if isinstance(expected,bool):
        return type(actual) is bool and actual is expected
    if isinstance(expected,(int,float)):
        return (type(actual) in (int,float) and math.isfinite(actual)
                and math.isclose(actual,expected,rel_tol=1e-8,abs_tol=1e-8))
    if isinstance(expected,list):
        return (isinstance(actual,list) and len(actual)==len(expected)
                and all(equivalent(a,b) for a,b in zip(actual,expected)))
    if isinstance(expected,dict):
        return (isinstance(actual,dict) and actual.keys()==expected.keys()
                and all(equivalent(actual[k],v) for k,v in expected.items()))
    raise TypeError('unsupported expected value')


def check(app, child):
    script=app/'solution.py'
    if not script.is_file():
        return False
    for data,expected,api in probes(child):
        # Copy just the public submission, preventing sample-module resolution
        # and stale import caches from bypassing the independent API probes.
        with tempfile.TemporaryDirectory(prefix='relu-probe-') as directory:
            work=Path(directory)
            shutil.copyfile(script,work/'solution.py')
            if api is not None:
                (work/'forward.py').write_text(api)
            try:
                result=subprocess.run([sys.executable,str(work/'solution.py')],cwd=work,
                    input=json.dumps(data),text=True,capture_output=True,timeout=2)
                if result.returncode or len(result.stdout)>65536:
                    return False
                actual=json.loads(result.stdout)
            except (subprocess.TimeoutExpired,ValueError,UnicodeError):
                return False
            if not equivalent(actual,expected):
                return False
    return True


if __name__=='__main__':
    app,oracle=map(Path,sys.argv[1:])
    child=json.loads((oracle/'task.json').read_text())['child']
    sys.exit(0 if check(app,child) else 1)
