"""Static cfg-union registration scan, using the baseline lexer. No Cargo."""
import re,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
OUT=Path(__file__).resolve().parent

def rel(p): return str(p.resolve().relative_to(ROOT))
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
result=dict(method='lexical cfg union; no macro expansion', edges=edges, unresolved=unresolved, reachable=sorted(scanned))
(OUT/'task-59-registrations.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
removed=[x['source'] for name in ['task-59-aliases.json','task-59-orphans.json'] for x in json.loads((OUT/name).read_text())]
assert not any(e['target'] in removed for e in edges)
print(json.dumps(dict(edges=len(edges),unresolved=len(unresolved),removed_targets_referenced=0)))

scope=json.loads((OUT.parent/'scope.json').read_text())
owners={p:row[0] for row in scope['rows'] for p in row[5]}
owners.update({p:59 for p in scope['residue']})
for row in scope.get('external_mounts',[]):
 if isinstance(row,dict):owners[row['path']]=row.get('owner_task')
external=[]
for name in sorted(scanned):
 if name.startswith('crates/vw-agent/src/') or not name.endswith('.rs'):continue
 external.append(dict(path=name,lines=len((ROOT/name).read_text().splitlines()),owner_task=owners.get(name),incoming=[e for e in edges if e['target']==name]))
(OUT/'task-59-external-current.json').write_text(json.dumps(dict(method='Static lib-reachable cfg-union source mounts; separate from host physical threshold; not measured compile time',source_files=len(external),source_lines=sum(x['lines'] for x in external),files=external),ensure_ascii=False,indent=2)+'\n')
print('external source mounts:',len(external),'lines:',sum(x['lines'] for x in external))
