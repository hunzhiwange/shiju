import json,sys
p=json.load(sys.stdin);d=p['dna'];ls=p['linkers']
ok=True
ok=ok and len(d)>=50 and set(d)<=set('ACGT') and all(15<=sum(c in 'GC' for c in d[i:i+50])<=35 for i in range(len(d)-49))
print(json.dumps(ok))
