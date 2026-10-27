import json,sys
v=json.load(sys.stdin)
a=v['values'];i=v['start'];total=0;visited=[]
while i<len(a):
    visited.append(i)
    if a[i]<0: break
    total+=a[i]
    i+=2 if a[i]==0 else 1
print(json.dumps({'total':total,'visited':visited,'stop':i}))
