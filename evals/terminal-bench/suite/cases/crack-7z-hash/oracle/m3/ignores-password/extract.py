import subprocess,sys,json,pathlib
pathlib.Path(sys.argv[4]).write_bytes(b'forged')
