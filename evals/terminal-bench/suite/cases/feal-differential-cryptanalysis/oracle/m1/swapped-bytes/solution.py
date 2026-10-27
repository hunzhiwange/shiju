def f(x):
    a, b = x & 255, (x >> 8) & 255
    t = (a + b + 1) & 255
    u = ((t << 3) | (t >> 5)) & 255
    v = (a ^ u ^ 0xA7) & 255
    return (v << 8) | u
import json,sys
d=json.load(sys.stdin);print(json.dumps([f(x) for x in d['words']]))
