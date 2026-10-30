import json,re,sys
from pathlib import Path
pos=[0.,0.,0.]; e=0.; absolute=True; erel=True; segments=[]; strokes=[]; current=[]
for raw in Path(sys.argv[1]).read_text().splitlines():
 line=raw.split(';')[0].strip().upper()
 tokens=re.findall(r'([A-Z])([+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+))',line)
 if not tokens: continue
 cmd=tokens[0][0]+str(int(float(tokens[0][1]))); args={k:float(v) for k,v in tokens[1:]}
 if cmd=='G90': absolute=True; continue
 if cmd=='G91': absolute=False; continue
 if cmd=='M82': erel=False; continue
 if cmd=='M83': erel=True; continue
 if cmd=='G92':
  for i,k in enumerate('XYZ'):
   if k in args: pos[i]=args[k]
  if 'E' in args: e=args['E']
  continue
 if cmd not in ('G0','G1'): continue
 end=[args.get(k,pos[i]) if absolute else pos[i]+args.get(k,0) for i,k in enumerate('XYZ')]
 delta=args.get('E',0) if erel else args.get('E',e)-e
 newe=e+delta if erel else args.get('E',e)
 draw=cmd=='G1' and delta!=0 and end!=pos
 if draw:
  segments.append([pos[:],end[:]])
  if not current: current=[pos[:]]
  current.append(end[:])
 else:
  if current: strokes.append(current); current=[]
 pos=end;e=newe
if current: strokes.append(current)
print(json.dumps(segments))
