"""Run submitted JSON programs against independent synthetic numeric probes."""
import json
import math
from pathlib import Path
import subprocess
import sys


def probes(child):
    if child == 'm1':
        for a,v in [([[7]],[3]), ([[2,3],[-5,4]],[2,-3]),
                    ([[1,2,0],[-3,4,5],[6,0,-2]],[-2,3,4]), ([[0,0],[0,0]],[0,0])]:
            yield {'matrix':a,'vector':v}, {'shape':[len(a),len(a)],
                'product':[sum(row[j]*v[j] for j in range(len(v))) for row in a],
                'norm':math.hypot(*v)}
    elif child == 'm2':
        for a,value in [([[-7,0],[0,2]],[-7,0]), ([[0,-3],[3,0]],[0,3]),
                        ([[4,2],[0,-1]],[4,0]), ([[2,1],[1,2]],[3,0]),
                        ([[0,0],[0,0]],[0,0]), ([[5,0],[0,-5]],[5,0]),
                        ([[1,-2],[2,1]],[1,2])]:
            yield a,value
    elif child == 'm3':
        for diagonal,index in [([0],0), ([0,0],0), ([3,3,1],0),
                               ([-6,2],0), ([2,-8,1],1), ([5,-5],0),
                               ([1,2,3,4,9],4), ([1e-12,-2e-12],1)]:
            a=[[v if i==j else 0 for j in range(len(diagonal))] for i,v in enumerate(diagonal)]
            yield {'matrix':a},{'value':[diagonal[index],0],
                                  'vector':[int(i==index) for i in range(len(diagonal))]}
    elif child == 'm4':
        h=math.sqrt(0.5)
        for a,l,v,wanted in [
            ([[3,0],[0,1]],[3,0],[[1,0],[0,0]],True),
            ([[3,0],[0,1]],[3,0],[[0,1],[0,0]],True),
            ([[0,-1],[1,0]],[0,1],[[h,0],[0,-h]],True),
            ([[0,0],[0,0]],[0,0],[[0,0],[1,0]],True),
            ([[3,0],[0,1]],[3,0],[[2,0],[0,0]],False),
            ([[3,0],[0,1]],[3,0],[[0,0],[0,0]],False),
            ([[3,0],[0,1]],[2,0],[[1,0],[0,0]],False),
            ([[3,0],[0,1]],[3,0],[[h,0],[h,0]],False),
            ([[0,-1],[1,0]],[0,1],[[h,0],[0,h]],False),
            ([[1]],[float('nan'),0],[[1,0]],False),
            ([[1]],[1,0],[[float('inf'),0]],False)]:
            yield {'matrix':a,'value':l,'vector':v},wanted
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


def check(app,child):
    script=app/'solution.py'
    if not script.is_file():
        return False
    for data,wanted in probes(child):
        try:
            result=subprocess.run([sys.executable,str(script)],cwd=app,
                input=json.dumps(data),text=True,capture_output=True,timeout=2)
            if result.returncode or len(result.stdout)>65536:
                return False
            actual=json.loads(result.stdout)
        except (subprocess.TimeoutExpired,ValueError,UnicodeError):
            return False
        if not equivalent(actual,wanted):
            return False
    return True


if __name__=='__main__':
    app,oracle=map(Path,sys.argv[1:])
    child=json.loads((oracle/'task.json').read_text())['child']
    sys.exit(0 if check(app,child) else 1)
