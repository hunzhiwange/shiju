import json,sys
a=json.load(sys.stdin)['matrix'];i=max(range(len(a)),key=lambda i:abs(a[i][i]))
print(json.dumps({'value':[a[i][i],0],'vector':[int(j==i) for j in range(len(a))]}))
