import json, sys
from pathlib import Path
w = json.loads(Path(sys.argv[1]).read_text())
x = json.loads(Path(sys.argv[2]).read_text())
h = [max(0, sum(a*b for a,b in zip(row,x))+0) for row,bias in zip(w['w1'],w['b1'])]
y = [sum(a*b for a,b in zip(row,h))+0 for row,bias in zip(w['w2'],w['b2'])]
print(json.dumps(y))
