import csv,json,sys
from pathlib import Path
import pyarrow as pa
import pyarrow.parquet as pq
root=Path(sys.argv[1])
MODE="gold"
rows=json.loads((root/'records.json').read_text())
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
(root/'merged.json').write_text(json.dumps({'rows':output,'report':report},ensure_ascii=False))
