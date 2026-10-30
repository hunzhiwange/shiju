import json,sys
v=json.load(open(sys.argv[1]));state=v['seed'];w=float(v['initial']);ids=[]
for step in range(v['steps']):
 state=(1664525*state+1013904223)%4294967296
 i=state%len(v['targets']);ids.append(i);w-=v['lr']*(w-v['targets'][i])
print(json.dumps({'iterations':v['steps'],'indices':ids,'weight':w,'state':state}))
