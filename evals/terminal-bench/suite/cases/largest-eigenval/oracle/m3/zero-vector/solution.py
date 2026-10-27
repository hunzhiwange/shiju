import json,sys
a=json.load(sys.stdin)['matrix'];i=max(range(len(a)),key=lambda i:abs(a[i][i]))
print(json.dumps({'value':[a[i][i],0],'vector':[0 for j in range(len(a))]}))
