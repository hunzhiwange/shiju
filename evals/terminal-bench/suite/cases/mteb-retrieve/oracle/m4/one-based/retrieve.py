import json,sys
def solve(d):
    return d['documents'][d['index']-1]
print(json.dumps(solve(json.load(sys.stdin)),ensure_ascii=False))
