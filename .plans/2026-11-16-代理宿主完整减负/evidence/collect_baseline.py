"""Static inventory only: no Cargo, macro expansion or cfg evaluation."""
import json, re, subprocess
from pathlib import Path
from collections import Counter
ROOT = Path(__file__).resolve().parents[3]
PLAN = Path(__file__).resolve().parent.parent
OUT = PLAN / 'evidence'
def rel(p): return str(p.resolve().relative_to(ROOT))
def dump(name, data): (OUT/name).write_text(json.dumps(data, ensure_ascii=False, indent=2)+'\n')
baseline = json.loads((PLAN/'baseline.json').read_text())
scope = json.loads((PLAN/'scope.json').read_text())
owned = {p:r[0] for r in scope['rows'] for p in r[5]}
owned.update({p:59 for p in scope['residue']})
counts = Counter(p for r in scope['rows'] for p in r[5]); counts.update(scope['residue'])
assert set(counts)==set(baseline) and all(n==1 for n in counts.values())
current = {rel(p):len(p.read_text().splitlines()) for p in (ROOT/'crates/vw-agent/src').rglob('*.rs')}
assert all((ROOT/p).is_file() for p in counts)
assert all(len(list(PLAN.glob(str(n)+'-*.md')))==1 for n in range(2,61))
# Lexer skips comments and treats string contents as opaque, including raw strings.
lex = re.compile(r"""//[^\n]*|/\*.*?\*/|(?:b?r)(?P<h>\#*)".*?"(?P=h)|b?"(?:\\.|[^"\\])*"|b?'(?:\\.|[^'\\])'|\b\w+\b|[^\s]""", re.S)
edges=[]; scanned=set(); unresolved=[]; declarations=[]
def scan(p):
    key=rel(p)
    if key in scanned: return
    scanned.add(key)
    src=p.read_text(); ts=[m for m in lex.finditer(src) if not m[0].startswith(('//','/*'))]
    base=p.parent if p.name in ('lib.rs','mod.rs','main.rs') else p.with_suffix('')
    frames=[(base,[])]; pending=[]; i=0
    def record(kind,target,pos,attrs):
        e=dict(source=key,line=src.count('\n',0,pos)+1,kind=kind,target=rel(target),attributes=attrs,exists=target.is_file())
        edges.append(e)
        if not target.is_file(): unresolved.append(e)
        elif kind in ('mod','path','include'): scan(target)
    while i<len(ts):
        v=ts[i][0]
        if v=='#' and i+1<len(ts) and ts[i+1][0] in ('[','!'):
            j=i+1
            if ts[j][0]=='!': j+=1
            level=1; k=j+1
            while k<len(ts) and level:
                level+=(ts[k][0]=='[')-(ts[k][0]==']'); k+=1
            pending.append(src[ts[i].start():ts[k-1].end()]); i=k; continue
        if v=='mod' and i+2<len(ts) and ts[i+2][0] in (';','{'):
            name=ts[i+1][0]; attrs=frames[-1][1]+pending
            paths=[re.search(r'\bpath\s*=\s*"([^"]+)"',a) for a in pending]; paths=[m[1] for m in paths if m]
            if ts[i+2][0]=='{':
                frames.append(((frames[-1][0]/paths[-1]) if paths else frames[-1][0]/name,attrs)); pending=[]; i+=3; continue
            if paths:
                # At file scope #[path] is relative to the source file directory.
                target=(p.parent if len(frames)==1 else frames[-1][0])/paths[-1]
            else:
                a=frames[-1][0]/(name+'.rs'); b=frames[-1][0]/name/'mod.rs'; target=a if a.is_file() else b
            record('path' if paths else 'mod',target,ts[i].start(),attrs); pending=[]; i+=3; continue
        if v in ('include','include_str','include_bytes') and i+2<len(ts) and ts[i+1][0]=='!':
            j=i+2; level=0; k=j
            while k<len(ts):
                level+=(ts[k][0]=='(')-(ts[k][0]==')'); k+=1
                if level==0: break
            expr=src[ts[j].start():ts[k-1].end()]
            strings=re.findall(r'"([^"\n]*)"',expr)
            if 'CARGO_MANIFEST_DIR' in strings:
                target=ROOT/'crates/vw-agent'/''.join(strings[1:]).lstrip('/')
            elif len(strings)==1: target=p.parent/strings[0]
            else: unresolved.append(dict(source=key,line=src.count('\n',0,ts[i].start())+1,expression=expr)); i=k; continue
            record(v,target,ts[i].start(),frames[-1][1]+pending); i=k; continue
        if v=='{': frames.append((frames[-1][0],frames[-1][1]+pending)); pending=[]
        elif v=='}' and len(frames)>1: frames.pop(); pending=[]
        elif v==';': pending=[]
        # Attributes for functions/items should not leak into following declarations.
        elif v in ('fn','struct','enum','impl','use','const','static','type','trait'): pending=[]
        i+=1
scan(ROOT/'crates/vw-agent/src/lib.rs')
reachable=set(scanned)
# Scan all remaining source files too, but keep their registrations separate from lib reachability.
for key in sorted(current): scan(ROOT/key)
external=[e for e in edges if not e['target'].startswith('crates/vw-agent/src/')]
# Propagate host task ownership through external source edges.
assigned={}; changed=True
while changed:
    changed=False
    for e in edges:
        owner=owned.get(e['source'],assigned.get(e['source']))
        if owner is not None and e['target'] not in owned and e['exists']:
            old=assigned.get(e['target'])
            if old is not None and old!=owner:
                if e['kind'] in ('include_str','include_bytes'): continue
                raise ValueError(('conflicting external owners',e,old,owner))
            if old is None: assigned[e['target']]=owner; changed=True
external_scope=[dict(path=p,owner_task=n,kind='source' if p.endswith('.rs') else 'resource', consumer_tasks=sorted({owned.get(e['source'],assigned.get(e['source'])) for e in edges if e['target']==p} - {None})) for p,n in sorted(assigned.items())]
scope['external_mounts']=external_scope
(PLAN/'scope.json').write_text(json.dumps(scope,ensure_ascii=False,indent=2)+'\n')
dump('external-mounts.json',dict(method='lexical cfg-union; no macro expansion; include_str/bytes are resources, not Rust compilation units',edges=external,ownership=external_scope,unresolved=unresolved))
dump('registrations.json',dict(edges=edges,lib_reachable=sorted(reachable),orphan_candidates=sorted(set(current)-reachable),unresolved=unresolved))
# Full declaration/fixture index, no behavioral interpretation or payloads.
ports=[]; globals_=[]
for key in sorted(scanned):
    for n,line in enumerate((ROOT/key).read_text().splitlines(),1):
        if re.search(r'\bimpl\b.*\b(?:\w*Host|Host)\s+for\b',line): ports.append(dict(path=key,line=n,declaration=line.strip(),owner_task=owned.get(key,assigned.get(key))))
        if re.search(r'\bstatic\b.*(?:OnceLock|LazyLock|Mutex)|(?:set_var|remove_var)\(',line): globals_.append(dict(path=key,line=n,declaration=line.strip(),owner_task=owned.get(key,assigned.get(key))))
dump('host-ports-and-globals.json',dict(ports=ports,process_global_candidates=globals_))
# Dependencies are retained with TOML section, path and owning tasks; no Cargo metadata.
deps=[]; section=''; host=(ROOT/'crates/vw-agent/Cargo.toml').read_text(); root=(ROOT/'Cargo.toml').read_text()
for line in host.splitlines():
    if line.startswith('['): section=line
    m=re.match(r'([\w-]+)\s*=.*?path\s*=\s*"([^"]+)"',line)
    if m:
        name,path=m.groups(); target=(ROOT/'crates/vw-agent'/path).resolve()
        assert (target/'Cargo.toml').is_file(),path
        deps.append(dict(dependency=name,section=section,path=rel(target),scope_tasks=[r[0] for r in scope['rows'] if name in r[6]],workspace_member=('"'+rel(target)+'"') in root))
dump('dependencies.json',deps)
diff=dict(added=sorted(set(current)-set(baseline)),removed=sorted(set(baseline)-set(current)),line_changes=[dict(path=p,baseline=baseline[p],current=current[p]) for p in sorted(set(current)&set(baseline)) if current[p]!=baseline[p]])
dump('snapshot.json',dict(head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),baseline_files=len(baseline),baseline_lines=sum(baseline.values()),current_files=len(current),current_lines=sum(current.values()),differences=diff,unique_original_owners=True,task_paths_exist=True))
print(json.dumps(dict(files=len(current),lines=sum(current.values()),reachable=len(reachable),orphans=len(set(current)-reachable),external=len(external_scope),unresolved=unresolved,differences=diff),ensure_ascii=False,indent=2))
