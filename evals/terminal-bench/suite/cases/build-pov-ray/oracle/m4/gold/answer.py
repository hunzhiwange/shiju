import json,sys,struct
from pathlib import Path
def read(p):
 b=Path(p).read_bytes()
 if len(b)<18 or b[1]!=0 or b[2]!=2 or b[16]!=24 or b[17]&48 != 32: raise ValueError('unsupported TGA')
 w,h=struct.unpack_from('<HH',b,12); offset=18+b[0]
 if not w or not h or len(b)!=offset+w*h*3: raise ValueError('truncated TGA')
 return [w,h],b[offset:]
a,x=read(sys.argv[1]);b,y=read(sys.argv[2])
n=sum(x[i:i+3]!=y[i:i+3] for i in range(0,len(x),3)) if a==b else None
print(json.dumps({'same':a==b and n==0,'changed_pixels':n,'first_size':a,'second_size':b}))
