import json, sys
v=json.load(sys.stdin)['values']
m=sum(v)/len(v) if v else None
print(json.dumps(dict(mean=m,anomalies=[x-m for x in reversed(v)])))
