import json,sys
def solve(d):
    return str(d['index'])
print(json.dumps(solve(json.load(sys.stdin)),ensure_ascii=False))
