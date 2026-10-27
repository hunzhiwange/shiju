import json,sys,math
q=json.load(sys.stdin);x=q['x'];n=len(x);mean=sum(x)/n
var=sum((v-mean)**2 for v in x)/n
y=[(v-mean)/math.sqrt(var+1e-5)*g+b for v,g,b in zip(x,q['gamma'],q['beta'])]
z=[sum(w*v for w,v in zip(row,y))+b for row,b in zip(q['weight'],q['bias'])]
out=[a+0.5*v*(1+math.tanh(math.sqrt(2/math.pi)*(v+0.044715*v**3))) for a,v in zip(x,z)]
print(json.dumps(out))
