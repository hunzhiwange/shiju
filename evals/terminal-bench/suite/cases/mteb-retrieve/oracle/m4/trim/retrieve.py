import json,sys
def solve(d):
    return d['documents'][d['index']].strip()
print(json.dumps(solve(json.load(sys.stdin)),ensure_ascii=False))
