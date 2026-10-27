import json,sys
from forward import forward
p=json.load(sys.stdin)
print(json.dumps([forward(0) for x in p['queries']]))
