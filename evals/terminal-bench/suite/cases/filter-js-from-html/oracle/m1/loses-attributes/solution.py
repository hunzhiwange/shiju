from html.parser import HTMLParser
class Tokens(HTMLParser):
 def __init__(self):
  super().__init__(convert_charrefs=True); self.events=[]
 def handle_starttag(self,t,a): self.events.append(['start',t,[]])
 def handle_endtag(self,t): self.events.append(['end',t])
 def handle_startendtag(self,t,a): self.handle_starttag(t,a)
 def handle_data(self,d):
  if d: self.events.append(['text',d])
import json,sys
p=Tokens();p.feed(sys.stdin.read());p.close();print(json.dumps(p.events,ensure_ascii=False))
