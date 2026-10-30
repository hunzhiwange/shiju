import json,sys,math
v=json.load(sys.stdin)
r=[{'cell':[math.floor(x),math.floor(y)],'rgb':[int(255.99*c) for c in rgb]} for x,y,rgb in v]
print(json.dumps(r))
