import subprocess,sys,json,pathlib
print(json.dumps({'password':pathlib.Path(sys.argv[2]).read_text().splitlines()[0]}))
