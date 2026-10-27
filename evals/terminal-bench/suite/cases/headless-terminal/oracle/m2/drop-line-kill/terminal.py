import codecs
import errno
import fcntl
import os
import pty
import select
import signal
import struct
import termios
import time

class Terminal:
    def __init__(self, rows=24, cols=80):
        self.decoder = codecs.getincrementaldecoder('utf-8')()
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.environ['PS1'] = ''
            os.environ['PS2'] = ''
            os.environ['TERM'] = 'dumb'
            os.execvp('bash', ['bash', '--noprofile', '--norc', '-i'])
        self.resize(rows, cols)
    def send(self, data):
        raw = data.replace('\x15', '').encode('utf-8')
        while raw:
            raw = raw[os.write(self.fd, raw):]
    def read(self, timeout=0.2):
        end = time.monotonic() + timeout
        data = b''
        while time.monotonic() < end:
            if not select.select([self.fd], [], [], max(0, end-time.monotonic()))[0]:
                break
            try:
                part = os.read(self.fd, 4096)
            except OSError as error:
                if error.errno == errno.EIO: break
                raise
            if not part: break
            data += part
        return self.decoder.decode(data)
    def resize(self, rows, cols):
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack('HHHH', rows, cols, 0, 0))
    def close(self):
        if self.fd is None: return
        try: os.kill(self.pid, signal.SIGKILL)
        except ProcessLookupError: pass
        os.waitpid(self.pid, 0)
        os.close(self.fd)
        self.fd = None
