"""Real local sockets with synthetic bytes; never an official data/score run."""
import hashlib
import json
from pathlib import Path
import socket
import socketserver
import subprocess
import sys
import tempfile
import threading
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'compat/cache_proxy.py'
BODY = b'synthetic public archive\n'


class Upstream(socketserver.BaseRequestHandler):
    def handle(self):
        data = b''
        while b'\r\n\r\n' not in data:
            data += self.request.recv(4096)
        self.server.requests.append(data)
        self.request.sendall(b'HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\nUPSTREAM')


class Proxy(unittest.TestCase):
    def test_cache_scope_head_range_and_passthrough(self):
        with tempfile.TemporaryDirectory() as directory, socketserver.TCPServer(('127.0.0.1', 0), Upstream) as upstream:
            upstream.requests = []
            thread = threading.Thread(target=upstream.serve_forever, daemon=True)
            thread.start()
            archive = Path(directory) / 'archive'
            archive.write_bytes(BODY)
            receipt = Path(directory) / 'receipt.json'
            code = ("import importlib.util,sys;from pathlib import Path;"
                    "s=importlib.util.spec_from_file_location('proxy',sys.argv[1]);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);"
                    "m.LENGTH=int(sys.argv[4]);m.SHA256=sys.argv[5];m.MD5=sys.argv[6];m.UPSTREAM=('127.0.0.1',int(sys.argv[7]));"
                    "m.serve(Path(sys.argv[2]),Path(sys.argv[3]))")
            process = subprocess.Popen([sys.executable, '-c', code, str(SCRIPT), str(archive), str(receipt),
                                        str(len(BODY)), hashlib.sha256(BODY).hexdigest(), hashlib.md5(BODY).hexdigest(),
                                        str(upstream.server_address[1])], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                self.assertEqual(process.stdout.readline(), b'ready\n')
                instance = json.loads(receipt.read_text())
                self.assertEqual(instance['listen'], '127.0.0.1')
                def request(raw):
                    with socket.create_connection(('127.0.0.1', instance['port']), timeout=3) as client:
                        client.sendall(raw)
                        chunks = []
                        while block := client.recv(4096):
                            chunks.append(block)
                    return b''.join(chunks)
                url = b'http://www.cs.toronto.edu/~kriz/cifar-10-binary.tar.gz'
                response = request(b'GET '+url+b' HTTP/1.1\r\nHost: public\r\n\r\n')
                self.assertIn(b'Content-Length: '+str(len(BODY)).encode()+b'\r\n', response)
                self.assertEqual(response.split(b'\r\n\r\n')[1], BODY)
                self.assertEqual(request(b'HEAD '+url+b' HTTP/1.1\r\n\r\n').split(b'\r\n\r\n')[1], b'')
                self.assertEqual(upstream.requests, [])
                for raw in (b'GET '+url+b' HTTP/1.1\r\nRange: bytes=0-1\r\n\r\n',
                            b'GET http://example.invalid/file HTTP/1.1\r\n\r\n',
                            b'CONNECT example.invalid:443 HTTP/1.1\r\n\r\n'):
                    self.assertTrue(request(raw).endswith(b'UPSTREAM'))
                    self.assertEqual(upstream.requests[-1], raw)
            finally:
                process.terminate()
                process.communicate(timeout=5)
                upstream.shutdown()
                thread.join(timeout=5)

    def test_wrong_archive_rejected_before_listener(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / 'archive'
            archive.write_bytes(b'wrong')
            receipt = Path(directory) / 'receipt.json'
            for truncated in (True, False):
                if not truncated:
                    with archive.open('wb') as stream:
                        stream.truncate(170052171)
                result = subprocess.run([sys.executable, str(SCRIPT), '--archive', str(archive), '--receipt', str(receipt)],
                                        capture_output=True, timeout=5)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(b'length' if truncated else b'hash mismatch', result.stderr)
                self.assertFalse(receipt.exists())
