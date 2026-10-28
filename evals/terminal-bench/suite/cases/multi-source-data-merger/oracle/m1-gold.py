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
(root/'normalized.json').write_text(json.dumps(rows,ensure_ascii=False))
