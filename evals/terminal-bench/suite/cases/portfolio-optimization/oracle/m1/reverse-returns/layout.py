import json,sys
p=json.load(sys.stdin);n=len(p['weights']);a=p['matrix'];order=p['order']
m=[[a[i*n+j] if order=='row' else a[j*n+i] for j in range(n)] for i in range(n)]
json.dump({'weights':p['weights'],'matrix':m,'returns':p['returns'][::-1]},sys.stdout)
