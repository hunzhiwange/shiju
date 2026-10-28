import json,sys
def solve(d):
    return {'model':d['model']['name'],'revision':d['model']['revision'],'documents':[s for s in d['text'].splitlines() if s]}
print(json.dumps(solve(json.load(sys.stdin)),ensure_ascii=False))
