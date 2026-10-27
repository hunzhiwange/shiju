import json,sys
d=json.load(sys.stdin);print(json.dumps(d['candidates']))
