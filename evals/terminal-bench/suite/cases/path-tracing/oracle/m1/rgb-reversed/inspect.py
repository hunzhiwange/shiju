import json,sys
v=json.load(sys.stdin)
t=' '.join(line.split('#')[0] for line in v['ppm'].splitlines()).split()
assert t[0]=='P3'
w,h,m=map(int,t[1:4]); a=list(map(int,t[4:])); assert len(a)==w*h*3
print(json.dumps({'width':w,'height':h,'pixels':[list(reversed(a[3*(y*w+x):3*(y*w+x)+3])) for x,y in v['queries']]}))
