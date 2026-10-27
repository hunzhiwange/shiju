import json,struct,sys
q=json.load(sys.stdin)
try:
 b=bytes.fromhex(q['hex']);pos=0
 def take(fmt):
  global pos
  n=struct.calcsize(fmt)
  if pos+n>len(b):raise ValueError()
  v=struct.unpack_from(fmt,b,pos);pos+=n;return v
 if take('<4s')[0]!=b'TNY1':raise ValueError()
 count=take('<I')[0];out={}
 if count>32:raise ValueError()
 for _ in range(count):
  n=take('<B')[0];name=take('<%ds'%n)[0].decode('ascii');a,c=take('<II')
  if not name or name in out or not 1<=a<=16 or not 1<=c<=16:raise ValueError()
  out[name]={'shape':[a,c],'values':list(take('<%df'%(a*c)))}
 if pos!=len(b):raise ValueError()
 print(json.dumps(out,allow_nan=False))
except (ValueError,struct.error,UnicodeError):print('{"error":"format"}')
