def solve(q):
 end=0;starts={q['capacity']};previous=False
 for b in q['blocks']:
  if type(b['free']) is not bool or b['words']<=0 or b['offset']!=end or False:return {'valid':False}
  starts.add(end);end+=b['words'];previous=b['free']
 return {'valid':end==q['capacity'] and q['cursor'] in starts}

import json,sys
print(json.dumps(solve(json.load(sys.stdin))))
