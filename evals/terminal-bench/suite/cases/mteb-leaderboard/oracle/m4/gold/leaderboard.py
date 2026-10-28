import json,sys
d=json.load(sys.stdin)
y=[r['organization'].strip()+'/'+r['model_name'].strip() for r in d['records']]
json.dump(y,sys.stdout,ensure_ascii=False)
