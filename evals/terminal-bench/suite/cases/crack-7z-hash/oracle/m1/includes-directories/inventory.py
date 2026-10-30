import subprocess,sys,json,pathlib
p=subprocess.run(['7z','l','-slt',sys.argv[1]],capture_output=True,text=True,check=True)
items=[]
for block in p.stdout.split('\n\n'):
 d=dict(line.split(' = ',1) for line in block.splitlines() if ' = ' in line)
 if 'Size' in d and d.get('Attributes','').startswith('A'):
  items.append({'path':d['Path'],'size':int(d['Size'])})
items.append({'path':'private','size':0})
print(json.dumps(sorted(items,key=lambda x:x['path'])))
