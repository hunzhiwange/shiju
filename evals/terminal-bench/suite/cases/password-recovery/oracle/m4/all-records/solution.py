import json, re, sys
from pathlib import Path
q=json.load(sys.stdin)
rows=json.loads(Path(q['source']).read_text()); values=sorted({r['password'] for r in rows}); Path(q['output']).write_text(''.join(s+'\n' for s in values),encoding='utf-8')
