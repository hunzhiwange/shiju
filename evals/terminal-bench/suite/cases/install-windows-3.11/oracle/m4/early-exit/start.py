import subprocess, sys
from pathlib import Path
app = Path(sys.argv[1])
args = ['qemu-system-i386', '-machine', 'pc,accel=tcg', '-m', '16',
        '-drive', 'file='+str(app/'boot.img')+',format=raw,if=floppy,snapshot=on',
        '-boot', 'a', '-vnc', '127.0.0.1:1',
        '-qmp', 'unix:'+str(app/'monitor.sock')+',server=on,wait=off',
        '-serial', 'file:'+str(app/'serial.log'), '-pidfile', str(app/'vm.pid'),
        '-daemonize', '-no-reboot']
subprocess.run(args, check=True, env={**__import__('os').environ, 'TMPDIR': '/tmp'})
import os, signal, time
pid=int((app/'vm.pid').read_text())
if os.fork()==0:
 time.sleep(1);os.kill(pid,signal.SIGTERM);os._exit(0)
