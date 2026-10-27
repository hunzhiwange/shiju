import json,sys,cmath
a=json.load(sys.stdin);t=a[0][0]+a[1][1];d=a[0][0]*a[1][1]-a[0][1]*a[1][0]
s=cmath.sqrt(t*t-4*d);v=max([(t+s)/2,(t-s)/2],key=lambda z:(-abs(z),z.real,z.imag))
print(json.dumps([v.real,v.imag]))
