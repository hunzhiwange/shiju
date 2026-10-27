import functools, http.server, ssl, sys
root, port, cert, key = sys.argv[1:]
class Handler(http.server.SimpleHTTPRequestHandler):
 def translate_path(self,path):
  self.directory = root+'/main'
  if path.startswith('/dev/'):
   self.directory = root+'/dev'; path=path[4:]
  return super().translate_path(path)
server=http.server.ThreadingHTTPServer(('127.0.0.1',int(port)),functools.partial(Handler,directory=root))
context=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER); context.load_cert_chain(cert,key)

server.serve_forever()
