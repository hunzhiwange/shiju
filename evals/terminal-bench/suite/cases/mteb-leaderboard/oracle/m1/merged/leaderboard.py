import json,sys
d=json.load(sys.stdin)
eligible=[s for s in d['snapshots'] if s['date']<=d['as_of']]
y=[row for s in eligible for row in s['rows']]
json.dump(y,sys.stdout,ensure_ascii=False)
