import json, re, subprocess, sys
from pathlib import Path
repo=Path(sys.argv[1])
def git(*args):
 return subprocess.check_output(['git','-C',str(repo),*args],stderr=subprocess.DEVNULL)
def objects():
 return [line.split() for line in git('cat-file','--batch-all-objects','--batch-check=%(objectname) %(objecttype)').decode().splitlines()]
print(json.dumps([{'oid':git('rev-parse','HEAD').decode().strip(),'type':'commit'}]))
