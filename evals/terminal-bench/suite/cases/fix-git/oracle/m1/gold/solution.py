import json, subprocess, sys
from pathlib import Path
r=Path(sys.argv[1])
def git(*args):
 return subprocess.check_output(['git','-C',str(r),*args],stderr=subprocess.DEVNULL).decode().strip()
lost=next(x.split()[0] for x in git('reflog','--format=%H %gs').splitlines() if 'commit: Recover profile' in x)
print(json.dumps(dict(master=git('rev-parse','master'),topic=git('rev-parse','topic'),lost=lost,lost_parents=git('rev-list','--parents','-n','1',lost).split()[1:],lost_on_master=subprocess.run(['git','-C',str(r),'merge-base','--is-ancestor',lost,'master'],capture_output=True).returncode==0)))
