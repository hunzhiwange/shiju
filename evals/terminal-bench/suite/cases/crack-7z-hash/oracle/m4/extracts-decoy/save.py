import subprocess,sys,json,pathlib
p=subprocess.run(['7z','x','-so','-p'+sys.argv[2],sys.argv[1],'secret_file.txt'],capture_output=True)
if p.returncode:sys.exit(1)
pathlib.Path(sys.argv[4]).write_bytes(p.stdout)
