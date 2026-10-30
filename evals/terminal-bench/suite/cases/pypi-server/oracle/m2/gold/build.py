from pathlib import Path
import zipfile,hashlib,base64
name='coppervec';version='0.4.2'
files={name+'/__init__.py':Path('src/coppervec/__init__.py').read_bytes(),name+'-'+version+'.dist-info/METADATA':('Metadata-Version: 2.1\nName: '+name+'\nVersion: '+version+'\n').encode(),name+'-'+version+'.dist-info/WHEEL':b'Wheel-Version: 1.0\nRoot-Is-Purelib: true\nTag: py3-none-any\n'}
record=''.join(p+',sha256='+base64.urlsafe_b64encode(hashlib.sha256(s).digest()).decode().rstrip('=')+','+str(len(s))+'\n' for p,s in files.items())
files[name+'-'+version+'.dist-info/RECORD']=(record+name+'-'+version+'.dist-info/RECORD,,\n').encode()
Path('dist').mkdir(exist_ok=True)
with zipfile.ZipFile('dist/'+name+'-'+version+'-py3-none-any.whl','w') as z:
    for p,s in files.items(): z.writestr(p,s)
