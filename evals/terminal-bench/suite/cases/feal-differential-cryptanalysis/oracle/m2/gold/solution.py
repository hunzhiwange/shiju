import json,sys
d=json.load(sys.stdin);print(json.dumps([{'a':x,'b':x^d['delta']} for x in d['plaintexts']]))
