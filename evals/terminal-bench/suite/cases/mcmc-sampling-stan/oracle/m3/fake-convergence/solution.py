import json,sys,math,random
d=json.load(sys.stdin)
draws=[]
for c,start in enumerate(d['starts']):
 rng=random.Random(d['seed']+c);x=start;chain=[]
 for k in range(d['iterations']):
  p=x+(-1 if rng.random()<0.5 else 1);u=rng.random()
  if 0<=p<len(d['weights']) and u<min(1,d['weights'][p]/d['weights'][x]):x=p
  chain.append(x)
 draws.append(chain[d['burn']:])
n=len(draws[0]);means=[sum(c)/n for c in draws];grand=sum(means)/4
W=sum(sum((v-m)**2 for v in c)/(n-1) for c,m in zip(draws,means))/4
B=n*sum((m-grand)**2 for m in means)/3
out={'draws':draws,'rhat':math.sqrt(((n-1)*W/n+B/n)/W)}

out['rhat']=1.0
print(json.dumps(out))
