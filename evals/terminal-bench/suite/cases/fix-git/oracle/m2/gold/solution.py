import json, subprocess, sys
from pathlib import Path
r=Path(sys.argv[1])
def git(*args):
 return subprocess.check_output(['git','-C',str(r),*args],stderr=subprocess.DEVNULL).decode().strip()
lost=next(x.split()[0] for x in git('reflog','--format=%H %gs').splitlines() if 'commit: Recover profile' in x)
git('checkout','master')
git('merge','--no-edit',lost)
