import json,sys
from forward import forward
p=json.load(sys.stdin)
print(json.dumps([forward(x) for x in p['queries']]))
