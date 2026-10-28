"""Independent black-box probes; expected values do not import candidate or gold."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile


PROBES = {'m1': [({'model': {'name': 'synthetic/embed-A', 'revision': 'abc123'}, 'text': '  Alpha  \n\n中文 beta\n'}, {'model': 'synthetic/embed-A', 'revision': 'abc123', 'documents': ['  Alpha  ', '', '中文 beta']}), ({'model': {'name': 'other/model', 'revision': 'def456'}, 'text': ''}, {'model': 'other/model', 'revision': 'def456', 'documents': []})], 'm2': [({'query': [1, 0], 'vectors': [[3, 0], [-2, 0], [0, 4], [0, 0], [3, 4]]}, [1, -1, 0, 0, 0.6]), ({'query': [0, 0], 'vectors': [[1, 2], [0, 0]]}, [0, 0]), ({'query': [1, 2], 'vectors': [[2, 4], [-1, -2]]}, [1, -1])], 'm3': [({'scores': [0.2, 0.9, 0.9, -0.1, 0.5], 'rank': 2}, 2), ({'scores': [0.2, 0.9, 0.9, -0.1, 0.5], 'rank': 5}, 3), ({'scores': [-3, -1, -1], 'rank': 1}, 1)], 'm4': [({'documents': ['first', '  北方 document  ', 'last'], 'index': 1}, '  北方 document  '), ({'documents': ['a', 'b', 'c'], 'index': 0}, 'a'), ({'documents': ['same', 'same', 'tail\ninside'], 'index': 2}, 'tail\ninside')]}

def probes(child):
    return PROBES[child]


def same(actual, expected):
    if type(actual) is not type(expected) and not (type(actual) in (int,float) and type(expected) in (int,float)):
        return False
    if isinstance(expected,list):
        return len(actual)==len(expected) and all(same(a,b) for a,b in zip(actual,expected))
    if isinstance(expected,dict):
        return actual.keys()==expected.keys() and all(same(actual[k],v) for k,v in expected.items())
    if isinstance(expected,(int,float)) and not isinstance(expected,bool):
        return isinstance(actual,(int,float)) and not isinstance(actual,bool) and abs(actual-expected)<=1e-9
    return actual==expected


def check(app, child):
    script=app/'retrieve.py'
    if not script.is_file():
        return False
    with tempfile.TemporaryDirectory(prefix='retrieve-probe-') as directory:
        for data,expected in probes(child):
            try:
                result=subprocess.run([sys.executable,str(script)],input=json.dumps(data),text=True,capture_output=True,cwd=directory,timeout=3)
                if result.returncode or len(result.stdout)>65536 or not same(json.loads(result.stdout),expected):
                    return False
            except (subprocess.TimeoutExpired,ValueError,UnicodeError):
                return False
    return True


if __name__=='__main__':
    app,oracle=map(Path,sys.argv[1:])
    sys.exit(0 if check(app,json.loads((oracle/'task.json').read_text())['child']) else 1)
