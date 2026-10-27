import json, subprocess, sys
from pathlib import Path
r=Path(sys.argv[1])
def git(*args):
 return subprocess.check_output(['git','-C',str(r),*args],stderr=subprocess.DEVNULL).decode().strip()
git('checkout','master')
subprocess.run(['git','-C',str(r),'merge','--no-commit','incoming'],capture_output=True)
(r/'status.txt').write_text('state: combined\n')
git('add','status.txt');git('commit','-m','Combine states')
(r/'master-only.txt').unlink()
git('add','.');git('commit','--amend','--no-edit')
