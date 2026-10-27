import json, re, subprocess, sys
from pathlib import Path
repo=Path(sys.argv[1])
def git(*args):
 return subprocess.check_output(['git','-C',str(repo),*args],stderr=subprocess.DEVNULL)
def objects():
 return [line.split() for line in git('cat-file','--batch-all-objects','--batch-check=%(objectname) %(objecttype)').decode().splitlines()]
recovered,manifest=map(Path,sys.argv[2:])
spec=json.loads(manifest.read_text())
recovered_ok=recovered.is_file() and not recovered.is_symlink() and recovered.read_bytes()==(spec['credential']+'\n').encode()
no_leak=all(b'secret[' not in git('cat-file','-p',oid) for oid,kind in objects())
files_ok=all((repo/name).is_file() and (repo/name).read_text()==value for name,value in spec['files'].items())
messages=git('log','--all','--format=%s').decode().splitlines()
preserved=messages==spec['messages']
print(json.dumps(dict(recovered_ok=recovered_ok,no_leak=no_leak,preserved=preserved,ready=recovered_ok and no_leak and preserved)))
