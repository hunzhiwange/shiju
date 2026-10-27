import json, subprocess, sys
from pathlib import Path
r=Path(sys.argv[1])
def git(*args):
 return subprocess.check_output(['git','-C',str(r),*args],stderr=subprocess.DEVNULL).decode().strip()
(r/'about.txt').write_text('profile: recovered\n')
git('add','about.txt');git('commit','-m','Copy profile')
