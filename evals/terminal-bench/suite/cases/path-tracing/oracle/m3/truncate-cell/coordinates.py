import json,sys,math
v=json.load(sys.stdin)
r=[{'cell':[int(x),int(y)],'rgb':[int(255.99*min(1,max(0,c))) for c in rgb]} for x,y,rgb in v]
print(json.dumps(r))
