"""Bounded synthetic MIPS32 interpreter; no host guest-syscall execution."""
import json
from pathlib import Path
import struct
import sys


def load(data):
    if len(data) < 52 or data[:7] != b'\x7fELF\x01\x01\x01':
        raise ValueError('ELF32 little endian required')
    h = struct.unpack_from('<HHIIIIIHHHHHH', data, 16)
    if h[0] != 2 or h[1] != 8 or h[2] != 1 or h[7] != 52 or h[8] != 32 or not 0 < h[9] <= 32:
        raise ValueError('invalid executable header')
    memory = {}
    for i in range(h[9]):
        off = h[4] + i * 32
        if off + 32 > len(data):
            raise ValueError('truncated program header')
        kind, offset, address, _, size, total, _, _ = struct.unpack_from('<8I', data, off)
        if kind != 1:
            continue
        if size > total or total > 1024 * 1024 or offset + size > len(data) or address + total > 2**32:
            raise ValueError('invalid segment')
        for j in range(total):
            memory[address + j] = data[offset + j] if j < size else 0
    if h[3] % 4 or any(h[3]+i not in memory for i in range(4)):
        raise ValueError('invalid entry')
    return memory, h[3]


def execute(data, payload=b'', kind='m4'):
    mem, pc = load(data)
    regs = [0] * 32
    regs[31] = 0xfffffffc
    if kind in ('m2', 'm3'):
        regs[4], regs[5] = 0x20000000, len(payload)
        mem.update({regs[4]+i: x for i,x in enumerate(payload)})
    pending = None
    opened = False
    closed = False
    file = bytearray()
    output = bytearray()
    def read(addr, count):
        if count > 1024*1024 or any(addr+i not in mem for i in range(count)):
            raise ValueError('unmapped read')
        return bytes(mem[addr+i] for i in range(count))
    for _ in range(10000):
        if pc == 0xfffffffc:
            return {'v0': regs[2], 'file': bytes(file), 'closed': closed, 'stdout': bytes(output)}
        if pc % 4:
            raise ValueError('unaligned instruction')
        word, = struct.unpack('<I', read(pc,4))
        op, rs, rt, rd = word>>26, (word>>21)&31, (word>>16)&31, (word>>11)&31
        imm = word&65535
        signed = imm if imm < 32768 else imm-65536
        branch = None
        if op == 0:
            fn=word&63
            if fn==0: regs[rd] = regs[rt] << ((word>>6)&31)
            elif fn==33: regs[rd] = regs[rs]+regs[rt]
            elif fn==8: branch = regs[rs]
            elif fn==12:
                call=regs[2]
                if call==2:
                    path=bytearray(); addr=regs[4]
                    for j in range(128):
                        char=read(addr+j,1)[0]
                        if not char: break
                        path.append(char)
                    if bytes(path)!=b'frame.bmp' or regs[5]!=577 or opened:
                        raise ValueError('invalid open')
                    opened=True; regs[2]=3
                elif call==1:
                    chunk=read(regs[5],regs[6])
                    if regs[4]==1: output.extend(chunk)
                    elif regs[4]==3 and opened and not closed: file.extend(chunk)
                    else: raise ValueError('bad write fd')
                    regs[2]=len(chunk)
                elif call==3 and regs[4]==3 and opened and not closed:
                    closed=True; regs[2]=0
                else: raise ValueError('unsupported syscall')
            else: raise ValueError('unsupported special')
        elif op==9: regs[rt]=regs[rs]+signed
        elif op==13: regs[rt]=regs[rs]|imm
        elif op==15: regs[rt]=imm<<16
        elif op in (4,5):
            taken=(regs[rs]==regs[rt]) if op==4 else (regs[rs]!=regs[rt])
            branch=pc+4+signed*4 if taken else pc+8
        elif op==2: branch=((pc+4)&0xf0000000)|((word&0x3ffffff)<<2)
        elif op in (32,35,36,40,43):
            addr=(regs[rs]+signed)&0xffffffff
            count=4 if op in (35,43) else 1
            if count==4 and addr%4: raise ValueError('unaligned word')
            if op in (32,35,36): regs[rt]=int.from_bytes(read(addr,count),'little',signed=(op==32))
            else:
                if any(addr+i not in mem for i in range(count)): raise ValueError('unmapped write')
                mem.update({addr+i:x for i,x in enumerate((regs[rt]&((1<<(8*count))-1)).to_bytes(count,'little'))})
        else: raise ValueError('unsupported opcode')
        regs=[x&0xffffffff for x in regs]; regs[0]=0
        if pending is not None and branch is not None: raise ValueError('branch in delay slot')
        pc,pending=(branch if branch is not None else pc+4),None
    raise ValueError('instruction budget exceeded')


if __name__ == '__main__':
    try:
        print(json.dumps({'v0': execute(Path(sys.argv[1]).read_bytes())['v0']}))
    except (ValueError, OSError, struct.error, IndexError):
        sys.exit(1)
