import json,sys
d=json.load(sys.stdin)
eligible=d['snapshots']
y=max(eligible,key=lambda s:s['date'])['rows'] if eligible else []
json.dump(y,sys.stdout,ensure_ascii=False)
