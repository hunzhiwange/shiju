import json,sys
d=json.load(sys.stdin)
y=[r['model'] for r in d['rows'] if bool(set(d['languages'])&set(r['languages'])) and all(type(r['scores'].get(t,{}).get(d['metric'])) in (int,float) for t in d['tasks'])]
json.dump(y,sys.stdout,ensure_ascii=False)
