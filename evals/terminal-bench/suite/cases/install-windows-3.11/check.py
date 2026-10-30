"""Observe actual QEMU, QMP, serial and framebuffer behavior of synthetic guests."""
import hashlib
import json
import os
from pathlib import Path
import signal
import shutil
import socket
import subprocess
import sys
import time
import uuid


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def wait_for(predicate, timeout, message):
    deadline = time.monotonic() + timeout
    while not predicate():
        assert time.monotonic() < deadline, message
        time.sleep(0.05)


class Monitor:
    def __init__(self, path):
        self.sock = socket.socket(socket.AF_UNIX)
        self.sock.settimeout(3)
        self.sock.connect(str(path))
        self.stream = self.sock.makefile('rwb', buffering=0)
        assert 'QMP' in json.loads(self.stream.readline())
        self.command('qmp_capabilities')

    def command(self, name, arguments=None):
        self.stream.write((json.dumps({'execute': name, 'arguments': arguments or {}})+'\n').encode())
        while True:
            result = json.loads(self.stream.readline())
            if 'event' in result:
                continue
            assert 'return' in result, result
            return result['return']

    def close(self):
        self.stream.close()
        self.sock.close()


def main():
    app, oracle = map(Path, sys.argv[1:])
    child = json.loads((oracle/'task.json').read_text())['child']
    if child not in ('m1', 'm2', 'm3', 'm4'):
        return 2
    if not shutil.which('qemu-system-i386'):
        return 2
    launcher = app/'start.py'
    if not launcher.is_file() or launcher.is_symlink():
        return 1
    before = digest(app/'boot.img')
    monitor = None
    pid = None
    try:
        result = subprocess.run([sys.executable, str(launcher), str(app)], cwd=app,
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=5)
        assert result.returncode == 0
        pid = int((app/'vm.pid').read_text())
        args = Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')
        assert Path(args[0].decode()).name == 'qemu-system-i386'
        monitor = Monitor(app/'monitor.sock')
        time.sleep(1)
        assert monitor.command('query-status')['running']
        if child != 'm1':
            serial = app/'serial.log'
            wait_for(lambda: serial.is_file() and b'SYNTHETIC_BOOT_READY\r\n' in serial.read_bytes(),
                     15, 'synthetic guest did not finish booting')
        if child == 'm1':
            drive = next(a.decode() for a in args if a.startswith(b'file='))
            assert 'file='+str(app/'boot.img')+',' in drive
            assert 'snapshot=on' in drive
            blocks = monitor.command('query-block')
            disk = next(b['inserted'] for b in blocks if 'inserted' in b)
            assert disk.get('backing_file') == str(app/'boot.img'), disk
            assert digest(app/'boot.img') == before
        elif child == 'm2':
            challenge = uuid.uuid4().hex[:8]
            offset = serial.stat().st_size
            for index, key in enumerate(challenge):
                monitor.command('send-key', {'keys': [{'type': 'qcode', 'data': key}],
                                             'hold-time': 20})
                expected = challenge[:index + 1].encode()
                wait_for(lambda: len(serial.read_bytes()[offset:]) >= len(expected),
                         3, 'guest did not echo the fresh keyboard challenge')
                assert serial.read_bytes()[offset:] == expected, 'incorrect guest keyboard echo'
                time.sleep(0.05)
        elif child in ('m3', 'm4'):
            with socket.create_connection(('127.0.0.1', 5901), timeout=3) as vnc:
                assert vnc.recv(12).startswith(b'RFB 003.')
            for index in range(3 if child == 'm4' else 1):
                assert monitor.command('query-status')['running']
                old = app/f'before-{index}.ppm'
                new = app/f'after-{index}.ppm'
                monitor.command('screendump', {'filename': str(old)})
                monitor.command('send-key', {'keys': [{'type': 'qcode', 'data': 'a'}]})
                time.sleep(1)
                monitor.command('screendump', {'filename': str(new)})
                assert old.read_bytes().startswith(b'P6\n')
                assert digest(old) != digest(new), 'keyboard input did not change guest display'
        assert digest(app/'boot.img') == before
        return 0
    except (AssertionError, OSError, ValueError, StopIteration, subprocess.TimeoutExpired) as error:
        print(type(error).__name__+': '+str(error), file=sys.stderr)
        return 1
    finally:
        if monitor:
            monitor.close()
        if pid:
            try:
                os.kill(pid, signal.SIGTERM)
            except ProcessLookupError:
                pass


if __name__ == '__main__':
    sys.exit(main())
