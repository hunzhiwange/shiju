import json,sys
from forward import forward
p=json.load(sys.stdin);g=p['grid'];y=[forward(x) for x in g]
s=[(y[i+1]-y[i])/(g[i+1]-g[i]) for i in range(len(g)-1)]
print(json.dumps([g[i] for i in range(1,len(g)-1) if abs(s[i]-s[i-1])>1e-8]))
