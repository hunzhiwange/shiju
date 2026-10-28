import json,sys
def solve(d):
    return {'model':d['model']['name'],'revision':'main','documents':d['text'].splitlines()}
print(json.dumps(solve(json.load(sys.stdin)),ensure_ascii=False))
