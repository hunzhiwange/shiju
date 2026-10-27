import json,sys,math
x=json.load(sys.stdin);a=x['matrix'];v=x['vector']
print(json.dumps({'shape':[1,len(a)],'product':[sum(t*z for t,z in zip(row,v)) for row in a],'norm':math.sqrt(sum(t*t for t in v))}))
