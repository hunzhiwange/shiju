import json,sys
def solve(d):
    q=d['query']; return [abs(sum(a*b for a,b in zip(q,v)))/(sum(a*a for a in q)*sum(b*b for b in v))**0.5 if any(v) and any(q) else 0.0 for v in d['vectors']]
print(json.dumps(solve(json.load(sys.stdin)),ensure_ascii=False))
