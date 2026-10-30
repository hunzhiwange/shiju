import json,sys,math,random
d=json.load(sys.stdin)
try:
 a,b=d['alpha'],d['beta'];w=d['warmup'];n=len(a[0])
 assert type(w) is int and 0<=w<n and len(a)==len(b)==4
 assert all(len(c)==n and all(type(x) in (int,float) and math.isfinite(x) and x>0 for x in c) for c in a+b)
 def stats(chains):
  v=sorted(x for c in chains for x in c[w:])
  def q(p):
   h=(len(v)-1)*p;i=int(h);return v[i]+(h-i)*(v[min(i+1,len(v)-1)]-v[i])
  return {'mean':sum(v)/len(v),'q05':q(.05),'q95':q(.95)}
 out={'alpha':stats(a),'beta':stats(b)}
except (AssertionError,IndexError,TypeError,ValueError):out=None

out['alpha']['mean']=out['alpha']['q05'] if out else 0
print(json.dumps(out))
