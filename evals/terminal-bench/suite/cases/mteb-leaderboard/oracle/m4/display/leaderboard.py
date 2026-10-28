import json,sys
d=json.load(sys.stdin)
y=[r['display'] for r in d['records']]
json.dump(y,sys.stdout,ensure_ascii=False)
