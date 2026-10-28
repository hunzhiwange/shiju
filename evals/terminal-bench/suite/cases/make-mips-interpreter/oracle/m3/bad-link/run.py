import json, sys
x=json.load(sys.stdin); words=x['words']; r=[0]*32; m=x['memory']; pc=0; pending=None; out=[]; code=None
for step in range(200):
    w=words[pc//4]; op=w>>26; a=(w>>21)&31; b=(w>>16)&31; imm=w&65535; s=imm-65536 if imm&32768 else imm
    target=None; old=pc
    if op==9: r[b]=(r[a]+s)&0xffffffff
    elif op in (4,5):
        take=(r[a]==r[b]) if op==4 else (r[a]!=r[b])
        if take: target=pc+4+4*s
    elif op in (2,3):
        target=(w&0x3ffffff)*4
        if op==3: r[31]=pc+4
    elif op==0 and (w&63)==8: target=r[a]
    elif w==0: pass
    elif w==12:
        if r[2]==4004:
            out+=m[r[5]:r[5]+r[6]]; r[2]=r[6]
        elif r[2]==4001: code=r[4]; pc+=4; break
        else: raise ValueError('unsupported syscall')
    else: raise ValueError('unsupported')
    r[0]=0
    pc=pending if pending is not None else old+4
    pending=target
else: raise ValueError('step limit')
print(json.dumps({'registers':r,'pc':pc,'stdout':out,'exit':code}))
