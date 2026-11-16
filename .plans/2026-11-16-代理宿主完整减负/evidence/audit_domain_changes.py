"""Static path accounting for scope domain packages; no Cargo."""
from pathlib import Path
import json,subprocess,re
ROOT=Path(__file__).resolve().parents[3]
PLAN=Path(__file__).resolve().parent.parent
OUT=PLAN/'evidence'
scope=json.loads((PLAN/'scope.json').read_text())
revision=json.loads((OUT/'snapshot.json').read_text())['head']
domains=sorted({name for row in scope['rows'] for name in row[6]})
paths=[str(p.relative_to(ROOT)) for name in domains if (p:=ROOT/'crates/libs'/name).is_dir()]
r=subprocess.run(['git','diff','--no-renames','--numstat',revision,'--',*paths],cwd=ROOT,text=True,capture_output=True,check=True)
changes={}
for line in r.stdout.splitlines():
 added,deleted,name=line.split('\t',2)
 if name.endswith('.rs') and added!='-':changes[name]=(int(added),int(deleted))
rows={}
for directory in paths:
 for p in sorted((ROOT/directory).rglob('*.rs')):
  key=str(p.relative_to(ROOT));now=len(p.read_text().splitlines());added,deleted=changes.get(key,(0,0))
  rows[key]=dict(path=key,before=now-added+deleted,after=now,added=added,deleted=deleted)
for key,(added,deleted) in changes.items():
 if key not in rows:rows[key]=dict(path=key,before=deleted-added,after=0,added=added,deleted=deleted)
app=[]
for directory in paths:
 for p in (ROOT/directory).rglob('*.rs'):
  for i,line in enumerate(p.read_text().splitlines(),1):
   if re.search(r'\bstruct\s+(?:App|AppState)\b',line):app.append(dict(path=str(p.relative_to(ROOT)),line=i,declaration=line.strip()))
(OUT/'task-59-domain-files.json').write_text(json.dumps(dict(method='Physical splitlines and git diff --numstat against baseline commit; configured scope destinations only, includes unrelated edits in these same packages; not credited production migration',revision=revision,domains=domains,files=list(rows.values()),app_state_candidates=app),ensure_ascii=False,indent=2)+'\n')
print(json.dumps(dict(domains=len(domains),files=len(rows),app_candidates=len(app))))
