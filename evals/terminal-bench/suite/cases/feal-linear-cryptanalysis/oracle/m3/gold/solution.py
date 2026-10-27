import json,sys
x=json.load(sys.stdin);a=[]
for k in x['candidates']:
 z=0
 for p,c in x['pairs']:
  v=(c+k)&255;v=((v<<2)|(v>>6))&255
  z+=(((p&x['plaintext_mask']).bit_count()^(v&x['output_mask']).bit_count())&1)==0
 a.append({'key':k,'zeros':z,'score':abs(2*z-len(x['pairs']))})
a.sort(key=lambda row:(-row['score'],row['key']))
print(json.dumps(a))
