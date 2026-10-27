import subprocess
class Terminal:
 def __init__(self, rows=24, cols=80):
  self.p=subprocess.Popen(['bash','--noprofile','--norc'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 def send(self,text): self.p.stdin.write(text.encode());self.p.stdin.flush()
 def read(self,timeout=0.2):
  import os,select
  return os.read(self.p.stdout.fileno(),65536).decode() if select.select([self.p.stdout],[],[],timeout)[0] else ''
 def close(self): self.p.kill();self.p.wait()
