"""Temporary loopback proxy for one hash-verified public CIFAR archive.

No TLS interception, request logging, global settings or dependency injection.
All non-target traffic is relayed through the existing local HTTP proxy.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import socket
import socketserver

URL = 'http://www.cs.toronto.edu/~kriz/cifar-10-binary.tar.gz'
SHA256 = 'c4a38c50a1bc5f3a1c5537f2155ab9d68f9f25eb1ed8d9ddda3db29a59bca1dd'
MD5 = 'c32a1d4ab5d03f1284b67883e8d87530'
LENGTH = 170052171
UPSTREAM = ('127.0.0.1', 7897)


def serve(archive, receipt):
    if archive.stat().st_size != LENGTH:
        raise ValueError('unexpected original archive length')
    hashes = [hashlib.sha256(), hashlib.md5()]
    with archive.open('rb') as stream:
        while block := stream.read(262144):
            for digest in hashes:
                digest.update(block)
    if [h.hexdigest() for h in hashes] != [SHA256, MD5]:
        raise ValueError('original archive hash mismatch')

    class Handler(socketserver.BaseRequestHandler):
        def handle(self):
            self.request.settimeout(30)
            data = b''
            while b'\r\n\r\n' not in data:
                chunk = self.request.recv(4096)
                if not chunk:
                    return
                data += chunk
                if len(data) > 65536:
                    return
            parts = data.split(b'\r\n', 1)[0].decode('ascii', 'strict').split()
            if len(parts) != 3:
                return
            method, url, _ = parts
            if url == URL and method in ('GET', 'HEAD') and b'\r\nrange:' not in data.lower():
                self.request.sendall(
                    b'HTTP/1.1 200 OK\r\nContent-Type: application/x-gzip\r\n'
                    b'Content-Length: '+str(LENGTH).encode()+b'\r\nETag: "a22ca4b-46b8be10162c1"\r\n'
                    b'Connection: close\r\n\r\n')
                if method == 'GET':
                    with archive.open('rb') as stream:
                        while chunk := stream.read(262144):
                            self.request.sendall(chunk)
                return
            with socket.create_connection(UPSTREAM, timeout=10) as upstream:
                upstream.sendall(data)
                sockets = [self.request, upstream]
                while True:
                    readable, _, _ = select.select(sockets, [], [], 30)
                    if not readable:
                        return
                    for sock in readable:
                        chunk = sock.recv(65536)
                        if not chunk:
                            return
                        (upstream if sock is self.request else self.request).sendall(chunk)

    class Server(socketserver.ThreadingTCPServer):
        allow_reuse_address = True
        daemon_threads = True

    with Server(('127.0.0.1', 0), Handler) as server:
        receipt.write_text(json.dumps({'pid': os.getpid(), 'port': server.server_address[1],
                                      'listen': '127.0.0.1', 'upstream': list(UPSTREAM),
                                      'cache_sha256': SHA256}) + '\n')
        print('ready', flush=True)
        server.serve_forever()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', required=True, type=Path)
    parser.add_argument('--receipt', required=True, type=Path)
    args = parser.parse_args()
    serve(args.archive.resolve(), args.receipt.resolve())
