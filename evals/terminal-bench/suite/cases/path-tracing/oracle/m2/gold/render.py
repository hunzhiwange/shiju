import json,sys,math
s=json.load(sys.stdin); out=[]
for y in range(s['height']):
 for x in range(s['width']):
  dx=x-s['cx'];dy=y-s['cy'];d=dx*dx+dy*dy
  out.extend(s['color'] if d<=s['radius']**2 else s['background'])
print('P3');print(s['width'],s['height']);print(255);print(*out)
