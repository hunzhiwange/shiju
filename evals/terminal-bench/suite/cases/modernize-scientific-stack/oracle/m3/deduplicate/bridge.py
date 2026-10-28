import csv, json, sys
from pathlib import Path
p=Path(json.load(sys.stdin)['path'])
g={}
with p.open(encoding='utf-8',newline='') as f:
    for r in csv.DictReader(f):
        if r['temperature'].strip():
            g.setdefault(r['station_id'],[]).append(float(r['temperature']))
print(json.dumps([dict(station_id=k,mean=sum(set(g[k]))/len(set(g[k]))) for k in sorted(g)]))
