import json,sys
v=json.load(sys.stdin)
total=v['seed']
for i,x in enumerate(v['values']):
    total=(total+(i+1)*x)%257
print(json.dumps(total))
