import json,sys,math,random
d=json.load(sys.stdin)
import csv,io
try:
 r=csv.DictReader(io.StringIO(d['csv']))
 assert set(r.fieldnames)=={'y','n'} and len(r.fieldnames)==2
 rows=list(r);assert rows
 y=[int(x['y']) for x in rows];n=[int(x['n']) for x in rows]
 assert all(0<=a<=b and b>0 for a,b in zip(y,n))
 out={'num_obs':len(y),'y':y,'n':n}
except (ValueError,AssertionError,TypeError,KeyError):out=None

out={'num_obs':sum(a>0 for a in y),'y':[a for a in y if a>0],'n':[b for a,b in zip(y,n) if a>0]}
print(json.dumps(out))
