import json,sys
def solve(d):
    return [sum(a*b for a,b in zip(d['query'],v)) for v in d['vectors']]
print(json.dumps(solve(json.load(sys.stdin)),ensure_ascii=False))
