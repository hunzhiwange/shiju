"""Build the public renderer dependency, never the parent solution/test package.

Run inside the digest-pinned compiler image, with a verified uncompressed
POVSRC.TAR in argv[1] and a writable build directory as the working directory.
"""
import hashlib
from pathlib import Path
import re
import subprocess
import sys
import tarfile

archive = Path(sys.argv[1])
EXPECTED = '0cff772e74f998c9ea9e94d6f6946d4a9f158ebc7ca35f856c0285c57b74c0ff'
if hashlib.sha256(archive.read_bytes()).hexdigest() != EXPECTED:
    raise ValueError('POVSRC.TAR identity mismatch')
with tarfile.open(archive) as source:
    for member in source.getmembers():
        if member.isfile() and member.name.startswith(('source/', 'machine/unix/')):
            Path(Path(member.name).name).write_bytes(source.extractfile(member).read())
config = Path('unixconf.h').read_text().replace(
    'char *malloc PARAMS((int size));', 'void unix_init_POVRAY(void);')
Path('config.h').write_text(config + '\n#define FINISH_POVRAY return 0;\n')
frame = Path('frame.h')
frame.write_text(frame.read_text().replace('#define MAIN_RETURN_TYPE void',
                                          '#define MAIN_RETURN_TYPE int'))
for path in Path('.').glob('*.c'):
    path.write_text('#include <stdlib.h>\n' + path.read_text())
unix = Path('unix.c')
unix.write_text(unix.read_text() + '\nstatic unsigned long rng_state=1;\n'
                'int pov_rand(void){rng_state=rng_state*1103515245UL+12345UL;'
                'return (rng_state/65536UL)&32767;}\n'
                'void pov_srand(int seed){rng_state=(unsigned long)seed;}\n')
objects = Path('unix.mak').read_text().split('POVOBJS =', 1)[1].split('\n\n', 1)[0]
names = re.findall(r'([a-z0-9]+)\.\$\(OBJ\)', objects) + ['unix']
subprocess.run(['gcc', '-std=gnu89', '-O2', '-DMAX_CONSTANTS=2000', '-o', 'povray',
                *[name + '.c' for name in names], '-lm'], check=True)
