"""Execute actual cross-built artifacts and independently challenge a VM loader."""
import importlib.util
import json
from pathlib import Path
import random
import shutil
import struct
import subprocess
import sys
import tempfile


def elf(words, entry=0x10000, tail=b''):
    text=b''.join(struct.pack('<I',w) for w in words)+tail
    header=b'\x7fELF\x01\x01\x01'+bytes(9)
    header+=struct.pack('<HHIIIIIHHHHHH',2,8,1,entry,52,0,0x50001000,52,32,1,40,0,0)
    return header+struct.pack('<8I',1,84,entry,entry,len(text),len(text)+16,7,4)+text


def main(app, oracle):
    spec=json.loads((oracle/'spec.json').read_text()); kind=spec['child']
    module=importlib.util.spec_from_file_location('trusted_vm',oracle/'reference_vm.py')
    vm=importlib.util.module_from_spec(module);module.loader.exec_module(vm)
    if kind=='m1':
        if not all(shutil.which(t) for t in ('mipsel-linux-gnu-as','mipsel-linux-gnu-ld')):
            raise RuntimeError('missing cross toolchain infrastructure')
        config=json.loads((app/'build.json').read_text())
        if set(config)!= {'assembler_args','linker_args'}: return False
        allowed={'-EL','-EB','-mips32','--64','-static','-e','main','0x10004','-Ttext','0x10000'}
        if any(not isinstance(args,list) or not args or len(args)>12 or
               any(not isinstance(a,str) or a not in allowed for a in args) for args in config.values()): return False
        with tempfile.TemporaryDirectory() as d:
            d=Path(d);shutil.copyfile(oracle/'probe.s',d/'probe.s')
            for command in [ ['mipsel-linux-gnu-as',*config['assembler_args'],'-o','probe.o','probe.s'],
                             ['mipsel-linux-gnu-ld',*config['linker_args'],'-o','probe.elf','probe.o'] ]:
                result=subprocess.run(command,cwd=d,capture_output=True,timeout=10)
                if result.returncode: return False
            return vm.load((d/'probe.elf').read_bytes())[1]==0x10000 and vm.execute((d/'probe.elf').read_bytes())['v0']==42
    if kind=='m2':
        data=(app/'program.elf').read_bytes()
        rng=random.Random(54)
        for payload in [b'',b'\x00',b'\xff',b'\x01\x80\x00\xfe',bytes(range(256)),bytes(rng.randrange(256) for _ in range(257))]:
            if vm.execute(data,payload,kind)['v0']!=sum(payload): return False
        return True
    if kind=='m3':
        data=(app/'program.elf').read_bytes()
        rng=random.Random(5403)
        for payload in [bytes(range(16)),bytes([255,0,128,0])*4,bytes(rng.randrange(256) for _ in range(16))]:
            result=vm.execute(data,payload,kind)
            expected=struct.pack('<2sIHHIIIIHHIIIIII',b'BM',70,0,0,54,40,2,0xfffffffe,1,32,0,16,0,0,0,0)+payload
            if not result['closed'] or result['file']!=expected: return False
        return True
    if kind!='m4': raise ValueError('unknown child')
    candidate=app/'vm.py'
    if not candidate.is_file(): return False
    rng=random.Random(5404)
    programs=[]
    for entry in [0x1000,0x20000,0x10400]:
        for _ in range(3):
            a=rng.randrange(1,32767); b=rng.randrange(1,32767)
            # addiu t0,0,a; addiu t1,0,b; addu v0,t0,t1; jr ra; addiu v0,v0,7
            programs.append((elf([0x24080000|a,0x24090000|b,0x01091021,0x03e00008,0x24420007],entry),a+b+7))
    # Taken and untaken branches, signed immediate, memory, byte zero extension.
    programs += [(elf([0x24080001,0x11080002,0x24020011,0x24020063,0x24420005,0x03e00008,0]),22),
                 (elf([0x2408ffff,0x15080002,0x24020003,0x24420009,0x03e00008,0]),12),
                 (elf([0x3c080001,0x35080020,0x91020000,0x03e00008,0,0,0,0],tail=b'\xff\x80\x00\x00'),255),
                 (elf([0x24080011,0x000810c0,0x03e00008,0]),136),
                 (elf([0x3c080001,0x35080040,0x2409fffe,0xad090000,0x8d020000,
                       0xa1000001,0x910a0001,0x004a1021,0x03e00008,0]+[0]*7),0xfffffffe),
                 (elf([0x24020000,0x08004004,0x24020011,0x24020063,0x03e00008,0]),17),
                 (elf([0x24000001,0x00001021,0x03e00008,0]),0),
                 (elf([0x3c080001,0x35080020,0x8d020000,0x03e00008,0,0,0,0]),0)]
    with tempfile.TemporaryDirectory() as directory:
        p=Path(directory)/'input.elf'
        for data,expected in programs:
            if vm.execute(data)['v0']!=expected: raise RuntimeError('trusted VM violates independent expected result')
            p.write_bytes(data)
            out=subprocess.run(['python3',str(candidate),str(p)],cwd=directory,capture_output=True,timeout=5)
            if out.returncode or json.loads(out.stdout)!={'v0':expected}: return False
        for data in [b'bad',programs[0][0][:60],programs[0][0][:5]+b'\x02'+programs[0][0][6:],programs[0][0][:18]+b'\x3e\x00'+programs[0][0][20:],elf([0xffffffff])]:
            p.write_bytes(data)
            out=subprocess.run(['python3',str(candidate),str(p)],cwd=directory,capture_output=True,timeout=5)
            if out.returncode==0: return False
    return True


if __name__=='__main__':
    try:
        accepted=main(Path(sys.argv[1]),Path(sys.argv[2]))
    except (FileNotFoundError,ValueError,KeyError,TypeError,struct.error,subprocess.TimeoutExpired):
        accepted=False
    sys.exit(0 if accepted else 1)
