import json,sys
r=json.load(sys.stdin)
print(json.dumps({'input':r['input']}))
