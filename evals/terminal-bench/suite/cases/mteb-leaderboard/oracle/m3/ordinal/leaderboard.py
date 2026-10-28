import json,sys
d=json.load(sys.stdin)
rows=sorted(d['rows'],key=lambda r:(-r['mean_task'],r['model']))
y=[]
for i,r in enumerate(rows):
 rank=i+1
 if rank<=d['k']:y.append(dict(model=r['model'],rank=rank))
json.dump(y,sys.stdout,ensure_ascii=False)
