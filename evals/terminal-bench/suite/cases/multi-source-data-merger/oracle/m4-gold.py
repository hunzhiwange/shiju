import csv,json,sys
from pathlib import Path
import pyarrow as pa
import pyarrow.parquet as pq
root=Path(sys.argv[1])
MODE="gold"
rows=[]
for src,data in [('source_a',json.loads((root/'source_a.json').read_text())),('source_b',list(csv.DictReader((root/'source_b.csv').open(newline='')))),('source_c',pq.read_table(root/'source_c.parquet').to_pylist())]:
 if MODE=='skip-parquet' and src=='source_c': continue
 for item in data:
  get=lambda *names: next((item[n] for n in names if n in item),None)
  status=get('status') if src=='source_a' else ('active' if str(get('is_active','active')).lower()=='true' else 'inactive')
  row={'source':src,'user_id':int(get('id','user_id','userId')),'name':get('full_name','name','userName'),'email':get('email','email_address'),'created_date':get('registration_date','created_at','joined'),'status':status}
  if MODE=='string-id': row['user_id']=str(row['user_id'])
  if MODE=='wrong-status' and src=='source_b': row['status']='active'
  rows.append(row)
users={}
ordered=sorted(rows,key=lambda r:['source_a','source_b','source_c'].index(r['source']),reverse=MODE=='reverse-priority')
for row in ordered: users.setdefault(int(row['user_id']),[]).append(row)
output=[];conflicts=[]
for uid,records in sorted(users.items()):
 result={'user_id':uid}
 for field in ['name','email','created_date','status']:
  values={r['source']:r.get(field) for r in records if r.get(field) is not None and not (MODE=='empty-as-null' and r.get(field)=='')}
  selected=next(iter(values.values()),None)
  result[field]=selected
  if len(set(values.values()))>1: conflicts.append({'user_id':uid,'field':field,'values':values,'selected':selected})
 output.append(result)
report={'total_conflicts':len(conflicts),'conflicts':conflicts}
if MODE=='drop-conflicts': report={'total_conflicts':0,'conflicts':[]}
if MODE=='drop-user': output=output[:-1]
if MODE=='string-output':
 for row in output: row['user_id']=str(row['user_id'])
pq.write_table(pa.Table.from_pylist(output),root/'merged_users.parquet')
(root/'conflicts.json').write_text(json.dumps(report,ensure_ascii=False))
