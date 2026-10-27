import json,sys,math
x=json.load(sys.stdin);a=x['matrix'];v=[complex(*z) for z in x['vector']];l=complex(*x['value'])
finite=all(math.isfinite(z) for row in a for z in row) and all(math.isfinite(z.real) and math.isfinite(z.imag) for z in v+[l])
n=math.sqrt(sum(abs(z)**2 for z in v));r=math.sqrt(sum(abs(sum(t*z for t,z in zip(row,v))-l*w)**2 for row,w in zip(a,v)))
scale=max(1,math.sqrt(sum(t*t for row in a for t in row)),abs(l))
print(json.dumps(bool(finite and abs(n-1)<=1e-8 and r<=1e-8*scale)))
