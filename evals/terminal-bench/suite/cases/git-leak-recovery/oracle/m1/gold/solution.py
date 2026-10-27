import json, re, subprocess, sys
from pathlib import Path
repo=Path(sys.argv[1])
def git(*args):
 return subprocess.check_output(['git','-C',str(repo),*args],stderr=subprocess.DEVNULL)
def objects():
 return [line.split() for line in git('cat-file','--batch-all-objects','--batch-check=%(objectname) %(objecttype)').decode().splitlines()]
reachable=set(git('rev-list','--objects','--all').decode().split())
print(json.dumps(sorted([{'oid':oid,'type':kind} for oid,kind in objects() if oid not in reachable],key=lambda x:x['oid'])))
