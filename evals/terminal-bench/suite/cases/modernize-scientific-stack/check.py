"""Independent numerical and CSV probes of submitted executable artifacts."""
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile


def probes(child, work):
    if child=='m1':
        for values,label,half,mean in [([1,4,2],'雪',1,7/3),([], 'ø',0,None),([-9,2,4,8],'007',2,1.25)]:
            yield {'values':values,'label':label},dict(label=label,half=half,mean=mean)
    elif child=='m2':
        for values,mean,anomalies in [([1,4,2],7/3,[-4/3,5/3,-1/3]),([],None,[]),([-4,0,7],1,[-5,-1,6]),([8],8,[0])]:
            yield {'values':values},dict(mean=mean,anomalies=anomalies)
    elif child=='m3':
        for i,(rows,expected) in enumerate([
            ('007,雪,-3\n007,雪,\n007,雪,9\n2,ø,1\n2,ø,1\n2,ø,7\n', [{'station_id':'007','mean':3},{'station_id':'2','mean':3}]),
            ('9,雨,\n01,雪,2.5\n01,雪,-1.5\n', [{'station_id':'01','mean':0.5}]),
            ('',[]),('b,ø,-4\na,雪,8\nb,ø,-2\n',[{'station_id':'a','mean':8},{'station_id':'b','mean':-3}])]):
            path=work/f'気候-{i}.csv'
            path.write_text('station_id,station_name,temperature\n'+rows,encoding='utf-8')
            yield {'path':str(path)},expected
    elif child=='m4':
        for old,new,atol,rtol,delta,maximum,ok in [
            ([0,10],[0.5,9],1,0,[0.5,-1],1,True),
            ([10],[11.05],0,0.1,[1.05],1.05,False),
            ([3,4],[1,1],0,0,[-2,-3],3,False),
            ([],[],0,0,[],0,True),
            ([1],[1,2],1,0,None,None,False),
            ([1],[1],0,0,[0],0,True)]:
            yield dict(old=old,new=new,atol=atol,rtol=rtol),dict(differences=delta,max_error=maximum,equivalent=ok)
    else:
        raise ValueError('unknown child')


def equal(actual, expected):
    if expected is None or isinstance(expected,bool):
        return actual is expected
    if isinstance(expected,(float,int)):
        return type(actual) in (float,int) and math.isfinite(actual) and math.isclose(actual,expected,rel_tol=1e-9,abs_tol=1e-9)
    if isinstance(expected,list):
        return isinstance(actual,list) and len(actual)==len(expected) and all(equal(a,b) for a,b in zip(actual,expected))
    if isinstance(expected,dict):
        return isinstance(actual,dict) and actual.keys()==expected.keys() and all(equal(actual[k],v) for k,v in expected.items())
    return type(actual) is type(expected) and actual==expected


def check(app,child):
    script=app/'bridge.py'
    if not script.is_file():
        return False
    with tempfile.TemporaryDirectory(prefix='scientific-probe-') as directory:
        work=Path(directory)
        for data,expected in probes(child,work):
            try:
                result=subprocess.run([sys.executable,str(script)],input=json.dumps(data),text=True,capture_output=True,cwd=work,timeout=3)
                if result.returncode or len(result.stdout)>65536 or not equal(json.loads(result.stdout),expected):
                    return False
            except (subprocess.TimeoutExpired,ValueError,UnicodeError):
                return False
        if child=='m1':
            result=subprocess.run([sys.executable,'-c',f'import sys;sys.path.insert(0,{str(app)!r});import bridge'],input='',text=True,capture_output=True,cwd=work,timeout=3)
            if result.returncode or result.stdout or result.stderr:
                return False
    return True


if __name__=='__main__':
    app,oracle=map(Path,sys.argv[1:])
    sys.exit(0 if check(app,json.loads((oracle/'task.json').read_text())['child']) else 1)
