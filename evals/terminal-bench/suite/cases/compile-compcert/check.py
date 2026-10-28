"""Independent executable checks for four narrow compiler tasks."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def run(args, cwd, expected=0, env=None):
    result = subprocess.run(args, cwd=cwd, env=env, capture_output=True, timeout=40)
    assert result.returncode == expected, (args, result.returncode, result.stderr[-1000:])
    return result.stdout


def executable(path):
    assert path.is_file() and os.access(path, os.X_OK), str(path)


def compiler_probe(compiler, work):
    executable(compiler)
    for factor in [91, 173]:
        source = work / f'input {factor}.c'
        binary = work / f'output {factor}'
        source.write_text('#ifndef __COMPCERT__\n#error genuine CompCert required\n#endif\n'
                          '#include <stdint.h>\n#include <stdio.h>\n#include <stdlib.h>\n'
                          'int main(int argc,char **argv){if(argc!=2)return 2;'
                          f'printf("%u\\n",(unsigned int)((uint32_t)strtoul(argv[1],0,10)*{factor}U));return 0;}}\n')
        run([str(compiler), str(source), '-o', str(binary)], work)
        executable(binary)
        assert binary.read_bytes()[:4] == b'\x7fELF'
        comments = run(['readelf', '-p', '.comment', str(binary)], work).lower()
        assert not any(signature in comments for signature in [b'clang', b'llvm', b'gnu c', b'gnu c++'])
        for value in [0, 1, 73, 4294967295]:
            assert run([str(binary), str(value)], work) == f'{value * factor & 0xffffffff}\n'.encode()
        run([str(binary)], work, 2)
    for label, content in [('invalid', 'int main( {'),
                           ('vla', 'int f(int n){int a[n];return sizeof(a);} int main(void){return f(3);}')]:
        source = work / f'{label}.c'
        source.write_text(content)
        outcome = subprocess.run([str(compiler), str(source), '-o', str(work / label)],
                                 cwd=work, capture_output=True, timeout=40)
        assert outcome.returncode != 0
    assert b'3.13' in run([str(compiler), '-version'], work)


def check(app, oracle):
    task = json.loads((oracle / 'task.json').read_text())
    for name, expected in task['public_sha256'].items():
        assert hashlib.sha256((app / name).read_bytes()).hexdigest() == expected
    mid = task['id']
    if mid == 'm1':
        entry = app / 'target.sh'
        executable(entry)
        for arch, target in [('x86_64', 'x86_64-linux'), ('amd64', 'x86_64-linux'),
                             ('aarch64', 'aarch64-linux'), ('arm64', 'aarch64-linux')]:
            assert run([str(entry), arch, 'Linux'], app) == (target + '\n').encode()
        for args in [[], ['x86_64'], ['x86_64', 'Darwin'], ['riscv64', 'Linux'],
                     ['armv7l', 'Linux'], ['x86_64', 'Linux', 'extra']]:
            assert run([str(entry), *args], app, 2) == b''
    elif mid == 'm2':
        entry = app / 'ccomp'
        if entry.exists() or entry.is_symlink():
            entry.unlink()
        run(['make'], app)
        executable(entry)
        assert entry.read_bytes()[:4] == b'\x7fELF'
        with tempfile.TemporaryDirectory(prefix='compiler-probe-', dir=app) as directory:
            compiler_probe(entry, Path(directory))
        run(['make', 'clean'], app)
        assert not entry.exists()
        run(['make'], app)
        executable(entry)
    elif mid == 'm3':
        source = app / 'rotate.c'
        assert source.is_file()
        with tempfile.TemporaryDirectory(prefix='rotate-probe-', dir=app) as directory:
            work = Path(directory)
            binary = work / 'rotate'
            run(['/usr/local/bin/ccomp', str(source), '-o', str(binary)], work)
            for value in [0, 1, 2, 0x80000000, 0xffffffff, 0x12345678, 0xabcdef01]:
                for count in [0, 1, 7, 16, 31]:
                    expected = value if count == 0 else ((value << count) | (value >> (32 - count))) & 0xffffffff
                    assert run([str(binary), str(value), str(count)], work) == f'{expected:08x}\n'.encode()
            for args in [[], ['1'], ['1','0','2'], ['-1','3'], ['4294967296','1'],
                         ['1','32'], ['1','-1'], ['bad','1'], ['1','1x'], ['', '0'], ['+1','0'], [' 1','0']]:
                assert run([str(binary), *args], work, 2) == b''
    elif mid == 'm4':
        installer = app / 'install.sh'
        executable(installer)
        with tempfile.TemporaryDirectory(prefix='install-probe-', dir=app) as directory:
            work = Path(directory)
            run([str(installer)], work)
            run([str(installer)], work)
            entry = app / 'install/bin/ccomp'
            env = {**os.environ, 'PATH': str(entry.parent) + ':' + os.environ['PATH']}
            assert run(['/bin/sh', '-c', 'command -v ccomp'], work, env=env) == (str(entry) + '\n').encode()
            compiler_probe(entry, work)
    else:
        raise ValueError('unknown task')
    return True


if __name__ == '__main__':
    try:
        check(Path(sys.argv[1]), Path(sys.argv[2]))
    except (AssertionError, OSError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
