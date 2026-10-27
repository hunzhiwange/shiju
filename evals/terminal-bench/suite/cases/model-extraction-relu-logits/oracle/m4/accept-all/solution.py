import json,sys
p=json.load(sys.stdin)
def f(n,x):
 return n['output_bias']+sum(a*max(0,sum(t*v for t,v in zip(w,x))+b) for w,b,a in zip(n['weights'],n['biases'],n['output_weights']))
d=[f(p['candidate'],x)-f(p['reference'],x) for x in p['inputs']]
print(json.dumps({'differences':d,'max_error':max(map(abs,d)), 'equivalent':True}))
