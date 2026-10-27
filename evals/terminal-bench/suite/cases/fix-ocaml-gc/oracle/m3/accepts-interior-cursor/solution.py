def solve(q):
 end=0;starts={q['capacity']};previous=False
 for b in q['blocks']:
  if type(b['free']) is not bool or b['words']<=0 or b['offset']!=end or (previous and b['free']):return {'valid':False}
  starts.add(end);end+=b['words'];previous=b['free']
 return {'valid':end==q['capacity'] and 0<=q['cursor']<=q['capacity']}

import json,sys
print(json.dumps(solve(json.load(sys.stdin))))
