def solve(q):
 return [dict(free=h<0,words=-h if h<0 else h) for h in q['headers']]

import json,sys
print(json.dumps(solve(json.load(sys.stdin))))
