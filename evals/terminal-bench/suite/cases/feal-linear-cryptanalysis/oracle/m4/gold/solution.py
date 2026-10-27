import json,sys
x=json.load(sys.stdin);out=[]
for c in x['ciphertexts']:
 l,r=c>>8,c&255
 for k in reversed(x['keys']):
  t=(l+k)&255;f=((t<<2)|(t>>6))&255
  l,r=r^f,l
 out.append((l<<8)|r)
print(json.dumps({'plaintexts':out,'matches':[p==q for p,q in zip(out,x['known_plaintexts'])]}))
