from pathlib import Path
import json, re, csv
from decimal import Decimal
files = {p.name:p.read_text() for p in Path('documents').iterdir()}
labels={n:('invoice' if re.match(r'(?:TAX )?INVOICE\b',t) else 'other') for n,t in files.items()}
def amounts(t):
    fields=dict(re.findall(r'^(Subtotal|Net|VAT|GST|Tax|Grand Total|Total|Amount Due): ([0-9]+\.[0-9]{2})$',t,re.M))
    return [next(fields[k] for k in ('Total','Grand Total','Amount Due') if k in fields),next((fields[k] for k in ('VAT','GST','Tax') if k in fields),'0.00')]
Path('invoices').mkdir(exist_ok=True)
Path('other').mkdir(exist_ok=True)
rows=[]
for n,t in sorted(files.items()):
    dest='invoices' if labels[n]=='invoice' else 'invoices'
    Path('documents',n).rename(Path(dest,n))
    if dest=='invoices': rows.append([n,*amounts(t)])
with Path('invoices/summary.csv').open('w',newline='') as f:
    w=csv.writer(f); w.writerow(['filename','total_amount','vat_amount']); w.writerows(rows)
    w.writerow(['total',str(sum(Decimal(r[1]) for r in rows)),str(sum(Decimal(r[2]) for r in rows))])
