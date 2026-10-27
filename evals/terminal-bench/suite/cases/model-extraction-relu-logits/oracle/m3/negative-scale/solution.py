import json,math,sys
p=json.load(sys.stdin);rows=[]
for w,b,a in zip(p['weights'],p['biases'],p['output_weights']):
 n=-math.sqrt(sum(x*x for x in w))
 rows.append(([x/n for x in w],b/n,a*n))
rows.sort(key=lambda r: tuple(r[0])+tuple(r[1:]))
print(json.dumps({'weights':[r[0] for r in rows],'biases':[r[1] for r in rows], 'output_weights':[r[2] for r in rows],'output_bias':p['output_bias']}))
