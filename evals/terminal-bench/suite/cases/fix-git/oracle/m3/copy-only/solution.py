import json, subprocess, sys
from pathlib import Path
r=Path(sys.argv[1])
def git(*args):
 return subprocess.check_output(['git','-C',str(r),*args],stderr=subprocess.DEVNULL).decode().strip()
(r/'status.txt').write_text('state: combined\n');(r/'incoming-only.txt').write_text('incoming only\n')
git('add','.');git('commit','-m','Copy incoming')
