def solve(q):
 heap=q['heap'];out=[];offset=0
 while offset<len(heap):
  h=heap[offset];words=-h if h<0 else h+1
  if offset+words>len(heap):return {'error':'bounds'}
  out.append(dict(offset=offset,free=h<0,words=words));offset+=words
 return out

import json,sys
print(json.dumps(solve(json.load(sys.stdin))))
