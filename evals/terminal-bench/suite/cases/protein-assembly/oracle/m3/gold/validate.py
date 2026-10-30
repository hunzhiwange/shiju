import json,sys
p=json.load(sys.stdin);d=p['dna'];ls=p['linkers']
ok=len(ls)==4 and all(5<=len(s)<=20 and set(s)<=set('GS') for s in ls)
ok=ok and len(d)>=50 and set(d)<=set('ACGT') and all(15<=sum(c in 'GC' for c in d[i:i+50])<=35 for i in range(len(d)-49))
print(json.dumps(ok))
