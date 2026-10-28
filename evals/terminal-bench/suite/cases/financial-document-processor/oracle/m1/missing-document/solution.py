from pathlib import Path
import json, re, csv
from decimal import Decimal
files = {p.name:p.read_text() for p in Path('documents').iterdir()}
labels={n:('invoice' if re.match(r'(?:TAX )?INVOICE\b',t) else 'other') for n,t in files.items()}
Path('result.json').write_text(json.dumps(labels))
r=json.loads(Path('result.json').read_text());r.pop('d.txt');Path('result.json').write_text(json.dumps(r))
