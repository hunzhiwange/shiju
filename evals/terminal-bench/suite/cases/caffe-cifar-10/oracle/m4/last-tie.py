import json,sys,struct
from pathlib import Path
v=json.load(open(sys.argv[1]));rows=v['weights'];bias=v['bias'];c=len(rows);d=len(rows[0]);out=Path(sys.argv[2])
flat=[a for row in rows for a in row]+bias
out.write_bytes(b'VWCF'+struct.pack('<II',c,d)+struct.pack('<'+'f'*len(flat),*flat))
b=out.read_bytes();c,d=struct.unpack('<II',b[4:12]);a=struct.unpack('<'+'f'*(c*d+c),b[12:]);x=v['x']
z=[sum(a[i*d+j]*x[j] for j in range(d))+a[c*d+i] for i in range(c)]
print(json.dumps({'logits':z,'prediction':max(range(c),key=lambda i:(z[i],i))}))
