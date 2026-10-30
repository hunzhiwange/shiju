"""Execute independent synthetic probes; no parent material or model trajectories."""
import json
import math
from pathlib import Path
import random
import statistics
import subprocess
import sys


def probes(child):
    if child == 'm1':
        for csv, expected in [
            ('y,n\n0,5\n2,7\n', {'num_obs': 2, 'y': [0, 2], 'n': [5, 7]}),
            ('n,y\n9,8\n3,0\n6,4\n', {'num_obs': 3, 'y': [8, 0, 4], 'n': [9, 3, 6]}),
            ('y,n\n1,1\n', {'num_obs': 1, 'y': [1], 'n': [1]}),
            ('y,n\n', None), ('y,n\n4,3\n', None), ('y,n\n-1,5\n', None),
            ('y,n\n0,0\n', None), ('y,n\n1.5,3\n', None),
            ('y,x\n1,3\n', None), ('y,y,n\n1,1,4\n', None)]:
            yield {'csv': csv}, expected
    elif child == 'm2':
        for a,b,y,n,t,p,u in [
            (2,3,[1],[4],[.3],[.4],.5),
            (1,1,[0,4],[5,5],[.1,.8],[.9,.2],.8),
            (3,2,[2,1],[8,3],[.4,.5],[.45,.3],.2),
            (.7,1.2,[0,2],[4,4],[.2,.6],[.25,.7],.95),
            (2,6,[1],[9],[.2],[.2001],.01)]:
            # Direct probability products are independent of submitted log-sum code.
            def density(theta):
                result = (a+b)**(-2.5)
                normalizer = math.gamma(a+b)/(math.gamma(a)*math.gamma(b))
                for yi,ni,x in zip(y,n,theta):
                    result *= math.comb(ni,yi)*x**yi*(1-x)**(ni-yi)
                    result *= normalizer*x**(a-1)*(1-x)**(b-1)
                return result
            current, proposed = density(t), density(p)
            accepted = u < min(1,proposed/current)
            yield {'alpha':a,'beta':b,'y':y,'n':n,'theta':t,'proposal':p,'u':u}, {
                'log_joint':math.log(current),'accepted':accepted,'theta':p if accepted else t}
    elif child == 'm3':
        for weights,starts,seed,count in [([1,2,1],[0,1,2,1],1,200),
                ([1,3,5,2],[0,3,1,2],17,240), ([2,1,4,3,1],[4,0,2,1],93,300)]:
            burn=count//2
            retained=[]
            for chain, start in enumerate(starts):
                generator=random.Random(seed+chain)
                state=start
                history=[]
                for _ in range(count):
                    delta=2*int(generator.random()>=.5)-1
                    candidate=state+delta
                    uniform=generator.random()
                    if candidate in range(len(weights)):
                        if uniform*weights[state]<weights[candidate]:
                            state=candidate
                    history.append(state)
                retained.append(history[burn:])
            within=statistics.mean(statistics.variance(c) for c in retained)
            between=statistics.variance([statistics.mean(c) for c in retained])
            rhat=math.sqrt((len(retained[0])-1)/len(retained[0])+between/within)
            yield {'weights':weights,'starts':starts,'seed':seed,'iterations':count,'burn':burn}, {
                'draws':retained,'rhat':rhat}
    elif child == 'm4':
        for a,b,w in [
            ([[90,1,2],[90,2,3],[90,3,4],[90,4,5]],[[80,5,6],[80,6,7],[80,7,8],[80,8,9]],1),
            ([[1,1,10],[2,2,3],[3,4,5],[7,8,99]],[[.1,.2,.3],[.2,.4,.8],[2,4,8],[3,6,9]],0),
            ([[7,2],[8,2],[9,2],[10,2]],[[4,3],[5,3],[6,3],[7,3]],1),
            ([[1,2]]*4,[[3,4]]*4,2),
            ([[1,float('nan')]]*4,[[3,4]]*4,0),
            ([[1,float('inf')]]*4,[[3,4]]*4,0),
            ([[1,2]]*4,[[3,4],[3],[3,4],[3,4]],0),
            ([[0,2]]*4,[[3,4]]*4,0)]:
            data={'alpha':a,'beta':b,'warmup':w}
            valid=(0<=w<len(a[0]) and all(len(c)==len(a[0]) and
                all(math.isfinite(x) and x>0 for x in c) for c in a+b))
            if not valid:
                yield data,None
                continue
            result={}
            for key,chains in [('alpha',a),('beta',b)]:
                values=sorted(sum((c[w:] for c in chains),[]))
                endpoints=[]
                for probability in [.05,.95]:
                    position=(len(values)-1)*probability
                    low,high=math.floor(position),math.ceil(position)
                    fraction=position-low
                    endpoints.append((1-fraction)*values[low]+fraction*values[high])
                result[key]={'mean':statistics.mean(values),'q05':endpoints[0],'q95':endpoints[1]}
            yield data,result
    else:
        raise ValueError('unknown child')


def equivalent(actual, expected):
    if expected is None:
        return actual is None
    if isinstance(expected,bool):
        return type(actual) is bool and actual is expected
    if isinstance(expected,(int,float)):
        return type(actual) in (int,float) and math.isfinite(actual) and math.isclose(actual,expected,rel_tol=1e-9,abs_tol=1e-9)
    if isinstance(expected,list):
        return isinstance(actual,list) and len(actual)==len(expected) and all(equivalent(x,y) for x,y in zip(actual,expected))
    if isinstance(expected,dict):
        return isinstance(actual,dict) and actual.keys()==expected.keys() and all(equivalent(actual[k],v) for k,v in expected.items())
    raise TypeError('unsupported expectation')


def check(app,child):
    script=app/'solution.py'
    if not script.is_file():
        return False
    for data,expected in probes(child):
        try:
            result=subprocess.run([sys.executable,str(script)],input=json.dumps(data),cwd=app,text=True,capture_output=True,timeout=3)
            if result.returncode or len(result.stdout)>65536 or not equivalent(json.loads(result.stdout),expected):
                return False
        except (ValueError,UnicodeError,subprocess.TimeoutExpired):
            return False
    return True


if __name__=='__main__':
    app,oracle=map(Path,sys.argv[1:])
    child=json.loads((oracle/'task.json').read_text())['child']
    sys.exit(0 if check(app,child) else 1)
