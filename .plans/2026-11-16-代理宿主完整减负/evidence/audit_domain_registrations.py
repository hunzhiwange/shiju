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
        e=dict(compiling_package=rel(manifest_root),source=key,line=src.count('\n',0,pos)+1,kind=kind,target=rel(target),attributes=attrs,exists=target.is_file())
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
                target=manifest_root/''.join(strings[1:]).lstrip('/')
            elif len(strings)==1: target=p.parent/strings[0]
            else: unresolved.append(dict(source=key,line=src.count('\n',0,ts[i].start())+1,expression=expr)); i=k; continue
            record(v,target,ts[i].start(),frames[-1][1]+pending); i=k; continue
        if v=='{': frames.append((frames[-1][0],frames[-1][1]+pending)); pending=[]
        elif v=='}' and len(frames)>1: frames.pop(); pending=[]
        elif v==';': pending=[]
        # Attributes for functions/items should not leak into following declarations.
        elif v in ('fn','struct','enum','impl','use','const','static','type','trait'): pending=[]
        i+=1


scope=json.loads((OUT.parent/'scope.json').read_text())
domains=sorted(p.parent.name for p in (ROOT/'crates/libs').glob('*/Cargo.toml'))
seen_all=set()
roots=[]
for name in domains:
 package=ROOT/'crates/libs'/name
 manifest_root=package
 scanned=set()
 if not package.is_dir():continue
 lib=package/'src/lib.rs'
 if lib.is_file():roots.append(dict(path=rel(lib),kind='lib'));scan(lib)
 for test in sorted((package/'tests').glob('*.rs')):
  roots.append(dict(path=rel(test),kind='integration'));scan(test)
 for test in sorted((package/'tests').glob('*/main.rs')):
  roots.append(dict(path=rel(test),kind='integration'));scan(test)
 manifest=package/'Cargo.toml'
 for block in re.split(r'(?m)^\[\[test\]\]\s*$',manifest.read_text())[1:]:
  block=re.split(r'(?m)^\[',block)[0]
  match=re.search(r'(?m)^path\s*=\s*"([^"]+)"',block)
  if match and (test:=package/match[1]).is_file():roots.append(dict(path=rel(test),kind='named_test'));scan(test)
 seen_all.update(scanned)
(OUT/'task-59-domain-registrations.json').write_text(json.dumps(dict(method='Lexical cfg union across scope domain lib/test roots; no Cargo or macro expansion',roots=roots,reachable=sorted(seen_all),edges=edges,unresolved=unresolved),ensure_ascii=False,indent=2)+'\n')
print(json.dumps(dict(roots=len(roots),reachable=len(seen_all),unresolved=len(unresolved))))
