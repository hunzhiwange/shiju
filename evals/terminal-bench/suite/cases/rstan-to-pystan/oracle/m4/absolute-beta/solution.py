import json,math,sys
from pathlib import Path
v=json.load(sys.stdin);s=v['draws'];means={}
for name,size in [('alpha',1),('sigma',1),('rho',v['D']),('beta',v['P']),('y_new',v['N_new'])]:
    rows=s[name]
    if len(rows)!=size: raise ValueError('shape')
    avg=[]
    for row in rows:
        if len(row)!=v['samples'] or not all(type(x) in (int,float) and math.isfinite(x) for x in row): raise ValueError('draws')
        avg.append(abs(sum(row)/len(row)))
    means[name]=avg
    Path(name+'_est.csv').write_text(''.join(format(x,'.17g')+'\n' for x in avg))
print(json.dumps({'means':means}))
