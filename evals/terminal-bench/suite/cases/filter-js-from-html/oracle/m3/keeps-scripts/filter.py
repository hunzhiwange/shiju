from html.parser import HTMLParser
class Tokens(HTMLParser):
 def __init__(self):
  super().__init__(convert_charrefs=True); self.events=[]
 def handle_starttag(self,t,a): self.events.append(['start',t,sorted([list(x) for x in a])])
 def handle_endtag(self,t): self.events.append(['end',t])
 def handle_startendtag(self,t,a): self.handle_starttag(t,a)
 def handle_data(self,d):
  if d: self.events.append(['text',d])
from html import escape
import sys
class Cleaner(HTMLParser):
 def __init__(self): super().__init__(convert_charrefs=True);self.out=[];self.skip=[]
 def handle_starttag(self,t,a):
  if self.skip:
   if t not in ('img','input','br','hr','embed','meta','link','frame'): self.skip.append(t)
   return
  if t in ('iframe','object','embed','frame'):
   if t not in ('embed','frame'): self.skip.append(t)
   return
  attrs=[]
  for k,v in a:
   if k.startswith('on'): continue
   if k in ('href','src','action','formaction','xlink:href') and v is not None:
    normalized=''.join(c for c in v if ord(c)>32).lower()
    if normalized.startswith(('javascript:','vbscript:','data:text/html')): continue
   attrs.append(' '+k if v is None else ' '+k+'="'+escape(v,quote=True)+'"')
  self.out.append('<'+t+''.join(attrs)+'>')
 def handle_endtag(self,t):
  if self.skip:
   if t==self.skip[-1]: self.skip.pop()
  else: self.out.append('</'+t+'>')
 def handle_startendtag(self,t,a): self.handle_starttag(t,a)
 def handle_data(self,d):
  if not self.skip:self.out.append(escape(d,quote=False))
if __name__=='__main__':
 if len(sys.argv)!=2:sys.exit(1)
 from pathlib import Path
 p=Path(sys.argv[1]); c=Cleaner(); c.feed(p.read_text(encoding='utf-8'));c.close();p.write_text(''.join(c.out),encoding='utf-8')
