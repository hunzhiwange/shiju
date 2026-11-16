"""Qualified compatibility references for forwarding removal; lexical navigation only."""
from pathlib import Path
import json,re
ROOT=Path(__file__).resolve().parents[3]
OUT=Path(__file__).resolve().parent
aliases=json.loads((OUT/'task-59-aliases.json').read_text())
rows=[]
for alias in aliases:
 name=alias['before'].split()[-1].rstrip(';')
 rows.append(dict(source=alias['source'],domain=alias['domain'],namespace=name,references=[]))
by_name={}
for row in rows:by_name.setdefault(row['namespace'],[]).append(row)
pattern=re.compile(r'\b('+'|'.join(re.escape(x) for x in by_name)+r')::')
for path in sorted((ROOT/'crates').rglob('*.rs')):
 for i,line in enumerate(path.read_text().splitlines(),1):
  if line.lstrip().startswith('//'):continue
  for match in pattern.finditer(line):
   for row in by_name[match[1]]:
    row['references'].append(dict(path=str(path.relative_to(ROOT)),line=i,source=line.strip()))
(OUT/'task-59-alias-callers.json').write_text(json.dumps(dict(method='Qualified namespace lexical references; imports may share names; not compiler-resolved calls. Alias targets retain existing behavior; zero new production algorithms credited.',mappings=rows),ensure_ascii=False,separators=(',',':'))+'\n')
print('alias mappings:',len(rows))
