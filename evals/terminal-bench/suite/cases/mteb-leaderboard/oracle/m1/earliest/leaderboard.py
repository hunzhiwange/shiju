import json,sys
d=json.load(sys.stdin)
eligible=[s for s in d['snapshots'] if s['date']<=d['as_of']]
y=min(eligible,key=lambda s:s['date'])['rows'] if eligible else []
json.dump(y,sys.stdout,ensure_ascii=False)
