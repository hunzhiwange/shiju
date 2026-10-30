import subprocess,sys,json,pathlib
p=subprocess.run(['7z','x','-so','-p'+sys.argv[2],sys.argv[1],sys.argv[3].split()[0]],capture_output=True)
listing=subprocess.run(['7z','l','-slt','-p'+sys.argv[2],sys.argv[1]],capture_output=True,text=True)
if p.returncode or ('Path = '+sys.argv[3]+'\n') not in listing.stdout:sys.exit(1)
pathlib.Path(sys.argv[4]).write_bytes(p.stdout)
