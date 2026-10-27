import json, re, subprocess, sys
from pathlib import Path
repo=Path(sys.argv[1])
def git(*args):
 return subprocess.check_output(['git','-C',str(repo),*args],stderr=subprocess.DEVNULL)
def objects():
 return [line.split() for line in git('cat-file','--batch-all-objects','--batch-check=%(objectname) %(objecttype)').decode().splitlines()]
values=set()
for oid,kind in objects():
 if kind=='blob':
  values.update(re.findall(rb'secret\[[^]\r\n]+\]',git('cat-file','blob',oid)))
if len(values)!=1: sys.exit(1)
Path(sys.argv[2]).write_bytes(next(iter(values))+b'\n')
git('reflog','expire','--expire=now','--all')
git('gc','--prune=now')
