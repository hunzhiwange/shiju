"""Map lexical baseline test declarations to current name candidates; not runtime registration."""
import json,re,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
PLAN=Path(__file__).resolve().parent.parent
OUT=PLAN/'evidence'
base=json.loads((PLAN/'baseline.json').read_text())
scope=json.loads((PLAN/'scope.json').read_text())
for item in scope.get('external_mounts',[]):
 if isinstance(item,dict) and item['path'].endswith('.rs'):base.setdefault(item['path'],0)
rev=json.loads((OUT/'snapshot.json').read_text())['head']
pattern=re.compile(r'#\[(?:[\w:]*::)?test(?:\([^\]]*\))?\]\s*(?:(?://[^\n]*\n|#\[[^\n]*\])\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)')
reachable=set()
for inventory in ['task-59-registrations.json','task-59-domain-registrations.json']:
 reachable.update(json.loads((OUT/inventory).read_text())['reachable'])
current={}
for root in ['crates/vw-agent/src','crates/libs']:
 for path in sorted((ROOT/root).rglob('*.rs')):
  text=path.read_text()
  for m in pattern.finditer(text):current.setdefault(m[1],[]).append(dict(path=str(path.relative_to(ROOT)),line=text.count('\n',0,m.start())+1,attributes=re.findall(r'#\[[^\n]*\]',m[0])))
scope=json.loads((PLAN/'scope.json').read_text())
owners={name:row[0] for row in scope['rows'] for name in row[5]}
owners.update({name:59 for name in scope['residue']})
owners.update({item['path']:item['owner_task'] for item in scope.get('external_mounts',[]) if isinstance(item,dict)})
rows=[]
for name in base:
 result=subprocess.run(['git','show',f'{rev}:{name}'],cwd=ROOT,text=True,capture_output=True)
 if result.returncode:raise RuntimeError(result.stderr)
 for m in pattern.finditer(result.stdout):
  all_candidates=current.get(m[1],[])
  candidates=[x for x in all_candidates if x['path'] in reachable]
  exact=[x for x in candidates if x['path']==name]
  rows.append(dict(source=name,name=m[1],line=result.stdout.count('\n',0,m.start())+1,attributes=re.findall(r'#\[[^\n]*\]',m[0]),candidates=candidates,unregistered_candidates=[x for x in all_candidates if x['path'] not in reachable],status='retained_same_path' if exact else ('unique_name_candidate' if len(candidates)==1 else ('ambiguous' if candidates else 'missing'))))
for row in rows:
 preferred=[x for x in row['candidates'] if x['path']==row['source']]
 if len(preferred)==1:row['selected']=preferred[0]
 elif len(row['candidates'])==1:row['selected']=row['candidates'][0]
 if row['status']=='ambiguous':
  owner=owners[row['source']]
  migration_owner=27 if owner==28 else owner
  selected=[x for x in row['candidates'] if re.search(rf'host_migration_{migration_owner}(?:_|/)', x['path'])]
  if len(selected)==1:
   row['selected']=selected[0]
   row['status']='owner_evidence_candidate'
counts={key:sum(x['status']==key for x in rows) for key in ['retained_same_path','unique_name_candidate','owner_evidence_candidate','ambiguous','missing']}
with (OUT/'task-59-test-candidates.json').open('w') as f:
 json.dump(dict(method='lexical standard test/tokio test attributes; excludes generated tests and intersected with lexical cfg-union lib/test reachability; does not prove assertion equivalence, cfg evaluation or compiler registration',baseline_commit=rev,counts=counts,tests=rows),f,ensure_ascii=False,separators=(',',':'))
print(json.dumps(counts))

ignored=[row for row in rows if any(a.startswith('#[ignore') for a in row.get('attributes',[]))]
mismatches=[]
for row in ignored:
 expected=[a for a in row['attributes'] if a.startswith('#[ignore')]
 if not any([a for a in candidate.get('attributes',[]) if a.startswith('#[ignore')]==expected for candidate in ([row['selected']] if 'selected' in row else row['candidates'])):mismatches.append(row)
(OUT/'task-59-ignore-attributes.json').write_text(json.dumps(dict(original_ignored_tests=len(ignored),mismatches=mismatches),ensure_ascii=False,indent=2)+'\n')
print('ignored attributes:',len(ignored),'mismatches:',len(mismatches))
