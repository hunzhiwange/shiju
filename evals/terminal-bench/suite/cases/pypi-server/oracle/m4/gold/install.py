import sys,subprocess
subprocess.run([sys.executable,'-m','pip','--isolated','install','--no-cache-dir','--no-deps','--only-binary=:all:','--index-url',sys.argv[1],'--target',sys.argv[2],'ambervec==0.4.2'],check=True)
