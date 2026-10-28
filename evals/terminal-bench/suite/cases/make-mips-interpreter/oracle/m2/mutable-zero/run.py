import json, sys
x=json.load(sys.stdin)
r=x['registers'][:]; r[0]=0
m=x['memory'][:]
for w in x['words']:
    op=w>>26; a=(w>>21)&31; b=(w>>16)&31; d=(w>>11)&31
    imm=w&65535; s=imm-65536 if imm&32768 else imm
    if op==0:
        fn=w&63
        if fn==33: v=r[a]+r[b]
        elif fn==35: v=r[a]-r[b]
        elif fn==42: v=int((r[a]^0x80000000)<(r[b]^0x80000000))
        else: raise ValueError('unsupported')
        r[d]=v&0xffffffff
    elif op==9: r[b]=(r[a]+s)&0xffffffff
    elif op==13: r[b]=r[a]|imm
    elif op==15: r[b]=imm<<16
    elif op in (32,36,35,40,43):
        addr=(r[a]+s)&0xffffffff
        if op==32: r[b]=(m[addr]-256 if m[addr]&128 else m[addr])&0xffffffff
        elif op==36: r[b]=m[addr]
        elif op==35: r[b]=int.from_bytes(bytes(m[addr:addr+4]),'little')
        elif op==40: m[addr]=r[b]&255
        else: m[addr:addr+4]=list(r[b].to_bytes(4,'little'))
    else: raise ValueError('unsupported')
    r[0]=r[0]
print(json.dumps({'registers':r,'memory':m}))
