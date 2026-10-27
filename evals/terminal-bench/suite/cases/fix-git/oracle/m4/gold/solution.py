import json, subprocess, sys
from pathlib import Path
r=Path(sys.argv[1])
def git(*args):
 return subprocess.check_output(['git','-C',str(r),*args],stderr=subprocess.DEVNULL).decode().strip()
def ok(*args):
 return subprocess.run(['git','-C',str(r),*args],capture_output=True).returncode==0
try: branch=git('symbolic-ref','--short','HEAD')
except subprocess.CalledProcessError: branch=''
try:
 content=(r/'profile.txt').read_bytes()==b'profile: ready\n' and subprocess.check_output(['git','-C',str(r),'show','HEAD:profile.txt'],stderr=subprocess.DEVNULL)==b'profile: ready\n'
except (OSError,subprocess.CalledProcessError): content=False
v=dict(on_master=branch=='master',clean=git('status','--porcelain')=='',content_ok=content,recovered=ok('merge-base','--is-ancestor','recovered-tip','master'))
v['ready']=all(v.values());print(json.dumps(v))
