import json,sys
from pathlib import Path
left,right=map(Path,sys.argv[1:])
d=[]
for n in ['ACCOUNTS.DAT','BOOKS.DAT']:
 a=left/n; b=right/n
 if not a.is_file() or not b.is_file() or a.read_bytes()!=b.read_bytes(): d.append(n)
print(json.dumps({'equal':not d,'differences':d}))
