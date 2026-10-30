import json, re, sys
from pathlib import Path
q=json.load(sys.stdin)
print(json.dumps(sorted({r['inode'] for r in q['records'] if r['links']==0 and r['path'].rsplit('/',1)[-1]=='launchcode.txt'})))
