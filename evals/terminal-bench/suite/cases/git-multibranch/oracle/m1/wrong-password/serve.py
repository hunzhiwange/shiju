import base64, functools, http.server, sys
from pathlib import Path
root, port = sys.argv[1], int(sys.argv[2])
class Handler(http.server.SimpleHTTPRequestHandler):
 def do_GET(self):
  if self.headers.get('Authorization') != 'Basic '+base64.b64encode(b'sandbox:wrong-pass').decode():
   self.send_response(401); self.send_header('WWW-Authenticate','Basic realm="sandbox"'); self.end_headers(); return
  super().do_GET()
http.server.ThreadingHTTPServer(('127.0.0.1',port),functools.partial(Handler,directory=root)).serve_forever()
