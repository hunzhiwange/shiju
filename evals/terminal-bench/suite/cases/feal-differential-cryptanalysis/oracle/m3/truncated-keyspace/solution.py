def f(x):
    a, b = x & 255, (x >> 8) & 255
    t = (a + b + 1) & 255
    u = ((t << 3) | (t >> 5)) & 255
    v = (a ^ u ^ 0xA7) & 255
    return (u << 8) | v
import json,sys
d=json.load(sys.stdin);print(json.dumps([k for k in d['candidates'][:-2] if all(f(o['x']^k)^f(o['y']^k)==o['difference'] for o in d['observations'])]))
