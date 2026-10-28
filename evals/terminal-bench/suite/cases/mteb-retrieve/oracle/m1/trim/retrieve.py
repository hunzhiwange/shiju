import json,sys
def solve(d):
    return {'model':d['model']['name'],'revision':d['model']['revision'],'documents':[s.strip() for s in d['text'].splitlines()]}
print(json.dumps(solve(json.load(sys.stdin)),ensure_ascii=False))
