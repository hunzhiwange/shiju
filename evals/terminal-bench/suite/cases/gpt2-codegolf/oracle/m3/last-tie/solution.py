import json,sys,math
q=json.load(sys.stdin);x=q['logits'];m=max(x);e=[math.exp(v-m) for v in x];s=sum(e)
print(json.dumps({'token':len(x)-1-list(reversed(x)).index(m),'probabilities':[v/s for v in e]}))
