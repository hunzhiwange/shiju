import json, sys
x=json.load(sys.stdin)
out=[]
for w in x['words']:
    imm=w & 65535
    out.append(dict(opcode=(w>>26)&63,rs=(w>>21)&31,rt=(w>>16)&31,rd=(w>>11)&31,shamt=(w>>6)&31,funct=w&63,immediate=imm,signed_immediate=imm-65536 if imm&32768 else imm,target=w&0x3ffffff))
print(json.dumps(out))
