import json,sys
def solve(d):
    return sorted(range(len(d['scores'])),key=lambda i:(-d['scores'][i],i))[min(d['rank'],len(d['scores'])-1)]
print(json.dumps(solve(json.load(sys.stdin)),ensure_ascii=False))
