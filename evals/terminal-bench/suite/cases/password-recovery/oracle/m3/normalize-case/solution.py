import json, re, sys
from pathlib import Path
q=json.load(sys.stdin)
print(json.dumps(sorted({s.upper() for s in q['candidates'] if re.fullmatch(r'8XD[A-Z0-9]{17}W54',s.upper())})))
