import csv, json, sys
from pathlib import Path
p=Path(json.load(sys.stdin)['path'])
g={}
with p.open(encoding='utf-8',newline='') as f:
    for r in csv.DictReader(f):
        if True:
            g.setdefault(r['station_id'],[]).append(float(r['temperature'] or 0))
print(json.dumps([dict(station_id=k,mean=sum(g[k])/len(g[k])) for k in sorted(g)]))
