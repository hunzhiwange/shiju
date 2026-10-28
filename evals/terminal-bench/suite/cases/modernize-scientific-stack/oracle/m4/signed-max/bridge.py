import json, sys
d=json.load(sys.stdin)
a,b=d['old'],d['new']
if len(a)!=len(b):
    o=dict(differences=None,max_error=None,equivalent=False)
else:
    delta=[y-x for x,y in zip(a,b)]
    o=dict(differences=delta,max_error=max(delta,default=0),equivalent=all(abs(y-x)<=d['atol']+d['rtol']*abs(x) for x,y in zip(a,b)))
print(json.dumps(o))
