"""Exercise actual candidate PTYs in an independent, bounded subprocess."""
import json
from pathlib import Path
import shutil
import subprocess
import sys

PROBE = r'''
import importlib.util
import os
from pathlib import Path
import shlex
import sys
import time
import uuid

spec = importlib.util.spec_from_file_location('candidate_terminal', sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
Terminal = module.Terminal
sessions = []

def create(**kwargs):
    t = Terminal(**kwargs)
    sessions.append(t)
    t.send('stty -echo\n')
    time.sleep(0.15)
    t.read(0.2)
    return t

def until(t, expected, timeout=5):
    text = ''
    deadline = time.monotonic() + timeout
    while expected not in text and time.monotonic() < deadline:
        value = t.read(0.1)
        assert isinstance(value, str)
        text += value
    assert expected in text, (expected[:80], text[:300])
    return text

def command(t, shell, expected):
    t.send(shell + '\n')
    return until(t, expected)

child = sys.argv[2]
try:
    t = create(rows=29, cols=93)
    nonce = uuid.uuid4().hex
    if child == 'm1':
        command(t, "test -t 0 && test -t 1 && case $- in *i*) printf 'PTY_OK\\n';; esac", 'PTY_OK')
        target = Path('/app/partial-' + nonce)
        t.send('printf ')
        t.send(shlex.quote(nonce))
        t.send(' > ' + str(target))
        time.sleep(0.15)
        assert not target.exists(), 'command ran before Enter'
        t.send('\n')
        command(t, 'cat ' + str(target), nonce)
        payload = 'AZ09_' * 4200
        command(t, "python3 -c " + shlex.quote("import sys;sys.stdout.write(" + repr(payload) + ")"), payload)
    elif child == 'm2':
        command(t, 'KEEP=' + nonce + "; cd /tmp; printf 'READY\\n'", 'READY')
        command(t, 'printf "%s:%s\\n" "$KEEP" "$PWD"', nonce + ':/tmp')
        target = Path('/app/cancel-' + nonce)
        t.send('sleep 2; printf bad > ' + str(target) + '\n')
        time.sleep(0.15)
        t.send('\x03')
        command(t, "printf 'RECOVERED\\n'", 'RECOVERED')
        time.sleep(2.1)
        assert not target.exists(), 'foreground process not interrupted'
        t.send('printf WRONG')
        t.send('\x15')
        t.send("printf 'CLEARED\\n'\n")
        output = until(t, 'CLEARED')
        assert 'WRONG' not in output, output
    elif child == 'm3':
        for rows, cols in [(29,93),(11,47),(37,121),(24,80)]:
            t.resize(rows,cols)
            command(t, 'stty size', str(rows)+' '+str(cols))
        text = '窗🙂é界' * 2800
        script = 'import sys;sys.stdout.write('+repr(text)+')'
        t.send('python3 -c ' + shlex.quote(script) + '\n')
        assert text in until(t,text)
        # Force the same UTF-8 code point across distinct read calls.
        script = "import os,time;os.write(1,b'\\xe7');time.sleep(0.6);os.write(1,b'\\xaa\\x97');time.sleep(0.6);os.write(1,b'END_SPLIT')"
        t.send('python3 -c ' + shlex.quote(script) + '\n')
        output = until(t, 'END_SPLIT')
        assert '窗END_SPLIT' in output, repr(output)
        t.send("printf '%s\\n' " + shlex.quote('输入🙂'+nonce) + '\n')
        until(t,'输入🙂'+nonce)
    elif child == 'm4':
        other = create()
        command(t,'VALUE=first_'+nonce+"; cd /tmp; printf 'ONE_READY\\n'",'ONE_READY')
        command(other,'VALUE=second_'+nonce+"; cd /app; printf 'TWO_READY\\n'",'TWO_READY')
        for _ in range(3):
            command(t,'printf "%s:%s\\n" "$VALUE" "$PWD"','first_'+nonce+':/tmp')
            command(other,'printf "%s:%s\\n" "$VALUE" "$PWD"','second_'+nonce+':/app')
        t.send("printf 'LEFT_")
        other.send("printf 'RIGHT_")
        t.send("SIDE\\n'\n")
        other.send("SIDE\\n'\n")
        left=until(t,'LEFT_SIDE');right=until(other,'RIGHT_SIDE')
        assert 'RIGHT_SIDE' not in left and 'LEFT_SIDE' not in right
        assert t.read(0.1) == '', 'read replayed consumed output'
        start=time.monotonic()
        assert other.read(0.1) == ''
        assert time.monotonic()-start < 0.8, 'idle timeout ignored'
        t.close();t.close();sessions.remove(t)
        command(other,"printf 'STILL_ALIVE\\n'",'STILL_ALIVE')
    else:
        raise RuntimeError('invalid checker child')
finally:
    for terminal in sessions:
        terminal.close()
'''


def main():
    app, oracle = map(Path, sys.argv[1:])
    child = json.loads((oracle / 'task.json').read_text())['child']
    if child not in ('m1', 'm2', 'm3', 'm4') or not shutil.which('bash'):
        return 2
    candidate = app / 'terminal.py'
    if not candidate.is_file() or candidate.is_symlink():
        return 1
    try:
        result = subprocess.run([sys.executable, '-c', PROBE, str(candidate), child],
                                cwd=app, capture_output=True, timeout=35)
    except subprocess.TimeoutExpired:
        return 1
    if result.returncode:
        sys.stderr.buffer.write(result.stderr[-4000:])
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
