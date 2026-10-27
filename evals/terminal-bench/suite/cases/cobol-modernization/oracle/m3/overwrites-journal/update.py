import json,sys
x=json.load(sys.stdin)
a=x['accounts']; b=x['items']; j=x['journal']
valid=any(v['id']==x['buyer'] for v in a) and any(v['id']==x['seller'] for v in a) and any(v['id']==x['item'] and v['owner']==x['seller'] for v in b)
if valid:
 for v in a:
  if v['id']==x['buyer']: v['balance']-=x['amount']
  if v['id']==x['seller']: v['balance']+=x['amount']
 for v in b:
  if v['id']==x['item']: v['owner']=x['buyer']
 j.clear()
 j.append(f"{x['item']}:{x['amount']}:{x['seller']}:{x['buyer']}")
print(json.dumps({'accounts':a,'items':b,'journal':j}))
