"""Independent black-box probes; expected values do not import candidate or gold."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def probes(child):
    if child == 'm1':
        snapshots = [dict(date='2025-09-01', rows=[{'model':'later/z','score':99}]),
                     dict(date='2025-07-01', rows=[{'model':'old/a','score':3}]),
                     dict(date='2025-08-31', rows=[{'model':'new/b','score':0}, {'model':'new/a','score':8}])]
        for date, expected in [('2025-08-31',[{'model':'new/b','score':0},{'model':'new/a','score':8}]),
                               ('2025-07-01',[{'model':'old/a','score':3}]),('2025-01-01',[])]:
            yield dict(snapshots=snapshots,as_of=date), expected
        yield dict(snapshots=[],as_of='2025-08-31'), []
    elif child == 'm2':
        rows = [dict(model='all/zero',languages=['da','sv','no','en'],scores={'a':{'ndcg':0,'map':99},'b':{'ndcg':70}}),
                dict(model='some/language',languages=['sv'],scores={'a':{'ndcg':99},'b':{'ndcg':99}}),
                dict(model='some/task',languages=['da','sv','no'],scores={'a':{'ndcg':99}}),
                dict(model='other/metric',languages=['da','sv','no'],scores={'a':{'map':99},'b':{'ndcg':99}}),
                dict(model='null/value',languages=['da','sv','no'],scores={'a':{'ndcg':None},'b':{'ndcg':99}}),
                dict(model='bool/value',languages=['da','sv','no'],scores={'a':{'ndcg':True},'b':{'ndcg':99}}),
                dict(model='all/negative',languages=['no','sv','da'],scores={'a':{'ndcg':-1},'b':{'ndcg':2}})]
        yield dict(rows=rows,tasks=['a','b'],languages=['da','sv','no'],metric='ndcg'), ['all/zero','all/negative']
        yield dict(rows=rows,tasks=['a'],languages=['sv'],metric='map'), ['all/zero','other/metric']
        yield dict(rows=[],tasks=['a'],languages=['sv'],metric='map'), []
    elif child == 'm3':
        rows=[dict(model='z/top',mean_task=80),dict(model='b/tie',mean_task=79.999),dict(model='a/tie',mean_task=79.999),dict(model='c/last',mean_task=1)]
        yield dict(rows=rows,k=2), [dict(model='z/top',rank=1),dict(model='a/tie',rank=2),dict(model='b/tie',rank=2)]
        yield dict(rows=list(reversed(rows)),k=1), [dict(model='z/top',rank=1)]
        yield dict(rows=[dict(model='b/x',mean_task=-2),dict(model='a/x',mean_task=-2),dict(model='c/x',mean_task=-3)],k=3), [dict(model='a/x',rank=1),dict(model='b/x',rank=1),dict(model='c/x',rank=3)]
        yield dict(rows=[],k=1), []
    elif child == 'm4':
        yield dict(records=[dict(display='Pretty Name',organization=' FjordAI ',model_name=' Nordic-v2.1 '),dict(display='Different',organization='FjordAI',model_name='Nordic-v2.1'),dict(display='x',organization=' ÅOrg ',model_name=' Embed_7B ')]), ['FjordAI/Nordic-v2.1','FjordAI/Nordic-v2.1','ÅOrg/Embed_7B']
        yield dict(records=[]), []
        yield dict(records=[dict(display='fake/model',organization='CaseOrg',model_name='M-0')]), ['CaseOrg/M-0']
    else:
        raise ValueError('unknown child')


def same(actual, expected):
    if type(actual) is not type(expected):
        return False
    if isinstance(expected,list):
        return len(actual)==len(expected) and all(same(a,b) for a,b in zip(actual,expected))
    if isinstance(expected,dict):
        return actual.keys()==expected.keys() and all(same(actual[k],v) for k,v in expected.items())
    return actual==expected


def check(app, child):
    script=app/'leaderboard.py'
    if not script.is_file():
        return False
    with tempfile.TemporaryDirectory(prefix='leaderboard-probe-') as directory:
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
