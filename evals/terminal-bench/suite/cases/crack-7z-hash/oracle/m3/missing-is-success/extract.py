import subprocess,sys,json,pathlib
p=subprocess.run(['7z','x','-so','-p'+sys.argv[2],sys.argv[1],sys.argv[3]],capture_output=True)
listing=subprocess.run(['7z','l','-slt','-p'+sys.argv[2],sys.argv[1]],capture_output=True,text=True)
if p.returncode:sys.exit(1)
pathlib.Path(sys.argv[4]).write_bytes(p.stdout)
