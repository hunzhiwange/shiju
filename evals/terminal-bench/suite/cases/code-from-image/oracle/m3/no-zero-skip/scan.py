import json,sys
v=json.load(sys.stdin)
a=v['values'];i=v['start'];total=0;visited=[]
while i<len(a):
    if a[i]<0: break
    visited.append(i)
    total+=a[i]
    i+=1
print(json.dumps({'total':total,'visited':visited,'stop':i}))
