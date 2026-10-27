import json,sys
store={}
results=[]
for op in json.load(sys.stdin):
    if op['op']=='set':
        store[op['key']]=abs(op['value'])
        results.append(abs(op['value']))
    else:
        results.append(store.get(op['key'],0))
print(json.dumps(results))
