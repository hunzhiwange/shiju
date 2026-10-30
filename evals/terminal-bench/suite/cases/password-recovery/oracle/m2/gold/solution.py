import json, re, sys
from pathlib import Path
q=json.load(sys.stdin)
data=Path(q['image']).read_bytes(); print(json.dumps({'hex':b''.join(data[e['offset']:e['offset']+e['length']] for e in sorted(q['extents'],key=lambda e:e['part'])).hex()}))
