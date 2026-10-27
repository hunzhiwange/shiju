import subprocess,sys
rounds,left,right=sys.argv[1:]
p=subprocess.run(["/usr/local/bin/pmars","-b","-r",rounds,"-f",left,right],capture_output=True,text=True)
sys.stdout.write(p.stdout);sys.stderr.write(p.stderr);sys.exit(p.returncode)
