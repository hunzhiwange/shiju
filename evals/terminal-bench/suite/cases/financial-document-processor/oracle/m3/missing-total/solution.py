from pathlib import Path
import json, re, csv
from decimal import Decimal
files = {p.name:p.read_text() for p in Path('documents').iterdir()}
def amounts(t):
    fields=dict(re.findall(r'^(Subtotal|Net|VAT|GST|Tax|Grand Total|Total|Amount Due): ([0-9]+\.[0-9]{2})$',t,re.M))
    return [next(fields[k] for k in ('Total','Grand Total','Amount Due') if k in fields),next((fields[k] for k in ('VAT','GST','Tax') if k in fields),'0.00')]
Path('result.json').write_text(json.dumps({n:amounts(t) for n,t in files.items()}))
r=json.loads(Path('result.json').read_text());r.pop('c.txt');Path('result.json').write_text(json.dumps(r))
