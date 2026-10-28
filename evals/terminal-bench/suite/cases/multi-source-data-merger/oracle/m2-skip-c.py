import csv,json,sys
from pathlib import Path
import pyarrow as pa
import pyarrow.parquet as pq
root=Path(sys.argv[1])
MODE="skip-c"
rows=json.loads((root/'records.json').read_text());groups={}
for row in rows:
 key=int(row['user_id'])
 if MODE=='by-name': key=row['name']
 if MODE=='skip-c' and row['source']=='source_c': continue
 groups.setdefault(key,{'user_id':int(row['user_id']),'sources':set()})['sources'].add(row['source'])
output=[{'user_id':v['user_id'],'sources':sorted(v['sources'])} for v in groups.values()]
if MODE=='no-normalize': output.append({'user_id':17,'sources':['source_b']})
(root/'groups.json').write_text(json.dumps(sorted(output,key=lambda r:r['user_id'])))
