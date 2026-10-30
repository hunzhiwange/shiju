import subprocess,sys,json,pathlib
for password in pathlib.Path(sys.argv[2]).read_text().splitlines():
 if '-' in password:continue
 p=subprocess.run(['7z','t','-p'+password,sys.argv[1]],capture_output=True)
 if p.returncode==0:
  print(json.dumps({'password':password}));sys.exit(0)
sys.exit(1)
