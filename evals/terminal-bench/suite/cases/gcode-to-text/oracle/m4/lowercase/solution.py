from pathlib import Path
import json,re,sys
font=json.loads(Path('/app/alphabet.json').read_text())
rows={}; x=y=0.
for line in Path(sys.argv[1]).read_text().splitlines():
 line=line.split(';')[0]; parts=line.split()
 if not parts or parts[0] not in ('G0','G1'):continue
 a={k:float(v) for k,v in re.findall(r'([XYE])([+-]?[0-9.]+)',line)}
 nx=a.get('X',x);ny=a.get('Y',y)
 if parts[0]=='G1' and a.get('E',0)>0:
  cell=int(round(x))//7;col=int(round(x))%7;row=6-int(round(y))
  rows.setdefault(cell,[['0']*5 for _ in range(7)])[row][col]='1'
 x,y=nx,ny
reverse={tuple(v):k for k,v in font.items()}
text=''.join(reverse[tuple(''.join(r) for r in rows[cell])] for cell in sorted(rows))
print(text.lower())
