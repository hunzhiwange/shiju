from html.parser import HTMLParser
class Tokens(HTMLParser):
 def __init__(self):
  super().__init__(convert_charrefs=True); self.events=[]
 def handle_starttag(self,t,a): self.events.append(['start',t,sorted([list(x) for x in a])])
 def handle_endtag(self,t): self.events.append(['end',t])
 def handle_startendtag(self,t,a): self.handle_starttag(t,a)
 def handle_data(self,d):
  if d: self.events.append(['text',d])
import hashlib,json,sys,subprocess,tempfile
from pathlib import Path
def unique(pairs):
 d={}
 for k,v in pairs:
  if k in d:raise ValueError('duplicate key')
  d[k]=v
 return d
def tokens(s):
 p=Tokens();p.feed(s);p.close();return p.events
def main():
 app,oracle=map(Path,sys.argv[1:]);spec=json.loads((oracle/'expected.json').read_text())
 program=app/('solution.py' if spec['mode']=='parse' else 'filter.py')
 if program.is_symlink() or not program.is_file():return 1
 originals={}
 for name,h in spec['hashes'].items():
  p=app/name
  if p.is_symlink() or not p.is_file():return 1
  originals[name]=p.read_bytes()
  if hashlib.sha256(originals[name]).hexdigest()!=h:return 1
 try:
  for name,data in originals.items():
   p=app/name
   if spec['interface']:
    p=app/('chosen path '+name);p.write_bytes(data)
   siblings={str(q):q.read_bytes() for q in app.glob('*.html') if q!=p}
   command=[sys.executable,'-I',str(program.resolve())]
   if spec['mode']=='filter':command.append(str(p.resolve()))
   with tempfile.TemporaryFile() as out,tempfile.TemporaryFile() as err:
    result=subprocess.run(command,input=data,stdout=out,stderr=err,cwd=app,timeout=5)
    if result.returncode or out.tell()>65536:return 1
    out.seek(0);raw=out.read()
   if p.is_symlink() or not p.is_file():return 1
   if any(Path(q).is_symlink() or not Path(q).is_file() or Path(q).read_bytes()!=b for q,b in siblings.items()):return 1
   if spec['mode']=='parse':
    actual=json.loads(raw,object_pairs_hook=unique)
    if p.read_bytes()!=data:return 1
   else:
    if p.stat().st_size>65536:return 1
    actual=tokens(p.read_text(encoding='utf-8'))
   if actual!=spec['outputs'][name]:return 1
   if spec['interface']:
    before=p.read_bytes()
    result=subprocess.run(command,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,cwd=app,timeout=5)
    if result.returncode or p.is_symlink() or not p.is_file() or p.read_bytes()!=before:return 1
  if spec['interface']:
   snapshot={str(q):q.read_bytes() for q in app.glob('*.html')}
   result=subprocess.run([sys.executable,'-I',str(program.resolve())],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,cwd=app,timeout=5)
   if result.returncode==0:return 1
   if any(Path(q).is_symlink() or not Path(q).is_file() or Path(q).read_bytes()!=b for q,b in snapshot.items()):return 1
  return 0
 except (ValueError,OSError,subprocess.TimeoutExpired):return 1
if __name__=='__main__':sys.exit(main())
