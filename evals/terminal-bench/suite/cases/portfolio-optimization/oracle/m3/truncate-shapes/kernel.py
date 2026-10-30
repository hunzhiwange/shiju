import json,sys,math
p=json.load(sys.stdin);w=p['weights'];s=p['matrix'];r=p['returns'];n=len(w)
n=min(n,len(r),len(s),*(len(row) for row in s)) if s else 0
if any(not math.isfinite(float(x)) for x in w+r+[v for row in s for v in row]):
    raise ValueError('finite')
t=[sum(s[i][j]*w[j] for j in range(n)) for i in range(n)]
q=sum(w[i]*t[i] for i in range(n))
if q<0: raise ValueError('negative variance')
json.dump({'risk':math.sqrt(q),'return':sum(w[i]*r[i] for i in range(n))},sys.stdout,allow_nan=False)
