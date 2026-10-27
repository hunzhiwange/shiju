import json,sys
d=json.load(sys.stdin);print(json.dumps([{'a':x,'b':x^128} for x in d['plaintexts']]))
