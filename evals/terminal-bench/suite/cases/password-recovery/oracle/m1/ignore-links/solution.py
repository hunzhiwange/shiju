import json, re, sys
from pathlib import Path
q=json.load(sys.stdin)
print(json.dumps(sorted({r['inode'] for r in q['records'] if r['allocated'] is False and r['path'].rsplit('/',1)[-1]=='launchcode.txt'})))
