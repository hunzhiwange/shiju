import json,math,sys

def main(v):
    d,p,c,seed=v['D'],v['P'],v['chains'],v['seed']
    if any(type(x) is not int or x<1 for x in [d,p,c]) or type(seed) is not int or seed!=1: raise ValueError()
    q=v['init']
    if set(q)!={'beta','alpha','rho','sigma'} or len(q['beta'])!=p or len(q['rho'])!=d: raise ValueError()
    nums=[*q['beta'],q['alpha'],*q['rho'],q['sigma']]
    if any(type(x) not in (int,float) or not math.isfinite(x) for x in nums): raise ValueError()
    if any(x<=0 for x in [q['alpha'],*q['rho'],q['sigma']]): raise ValueError()
    return {'parameter_shapes':{'beta':[p],'alpha':[],'rho':[d],'sigma':[]},
            'build':{'random_seed':seed},'sample':{'num_chains':c,'init':[q.copy() for _ in range(c)]}}
try: result=main(json.load(sys.stdin))
except (ValueError,TypeError,KeyError,IndexError): result={'error':'invalid-init'}
print(json.dumps(result,allow_nan=False))
