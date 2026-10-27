import json,sys
store={}
results=[]
for op in json.load(sys.stdin):
    if (not isinstance(op,dict) or op.get('op') not in ('set','get')
        or not isinstance(op.get('key'),str) or not op['key']
        or (op['op']=='set' and (type(op.get('value')) is not int
            or not -2147483648<=op['value']<=2147483647))):
        results.append({'error':'invalid'})
        continue
    if op['op']=='set':
        store[op['key']]=op['value']
        results.append({'val':op['value']})
    elif op['key'] not in store:
        results.append({'error':'missing'})
    else:
        results.append({'val':store[op['key']]})
print(json.dumps(results))
