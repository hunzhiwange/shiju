import json, sys
x=json.load(sys.stdin); a=x['left']; b=x['right']; diff=None
for field in ['registers','memory']:
    aa=a[field]; bb=b[field]
    for idx in range(max(len(aa),len(bb))):
        av=aa[idx] if idx<len(aa) else None
        bv=bb[idx] if idx<len(bb) else None
        if av!=bv:
            diff={'field':field,'index':idx,'left':av,'right':bv}; break
    if diff is not None: break
if diff is None and a['exit']!=b['exit']:
    diff={'field':'exit','index':None,'left':a['exit'],'right':b['exit']}
print(json.dumps({'equal':diff is None,'difference':diff}))
