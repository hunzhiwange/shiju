import json,sys,math,random
d=json.load(sys.stdin)
a,b=d['alpha'],d['beta']
def target(t):
 return -2.5*math.log(a+b)+sum(math.lgamma(n+1)-math.lgamma(y+1)-math.lgamma(n-y+1)+y*math.log(x)+(n-y)*math.log1p(-x)+math.lgamma(a+b)-math.lgamma(a)-math.lgamma(b)+(a-1)*math.log(x)+(b-1)*math.log1p(-x) for y,n,x in zip(d['y'],d['n'],t))
l=target(d['theta']);p=target(d['proposal'])
accept=math.log(d['u'])<min(0,p-l)
out={'log_joint':l,'accepted':accept,'theta':d['proposal'] if accept else d['theta']}

out['accepted']=True;out['theta']=d['proposal']
print(json.dumps(out))
