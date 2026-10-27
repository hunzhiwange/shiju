import json,sys
store={}
results=[]
for op in json.load(sys.stdin):
    if op['op']=='set':
        store[op['key'].lower()]=op['value']
        results.append(op['value'])
    else:
        results.append(store.get(op['key'].lower(),0))
print(json.dumps(results))
