def solve(q):
 blocks=[dict(offset=0,words=q['capacity'],free=True)];active={};results=[]
 for op in q['ops']:
  if op[0]=='alloc':
   need=op[2]+1;result=None
   for i,b in enumerate(blocks):
    if b['free'] and b['words']>=need:
     result=b['offset'];active[op[1]]=result;rest=b['words']-need
     blocks[i:i+1]=[dict(offset=result,words=need,free=False)]+([dict(offset=result+need,words=rest,free=True)] if rest else [])
     break
   results.append(result)
  else:
   offset=active.pop(op[1]);i=next(i for i,b in enumerate(blocks) if b['offset']==offset);blocks[i]['free']=True
   j=0
   while j+1<len(blocks):
    if False:
     blocks[j]['words']+=blocks[j+1]['words'];blocks.pop(j+1)
    else:j+=1
   results.append(None)
 return dict(results=results,blocks=blocks)

import json,sys
print(json.dumps(solve(json.load(sys.stdin))))
