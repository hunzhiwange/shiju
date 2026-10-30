import http.server,sys
class H(http.server.BaseHTTPRequestHandler):
 def do_GET(self):
  self.send_response(200);self.end_headers();self.wfile.write(b'index')
http.server.HTTPServer(('127.0.0.1',int(sys.argv[1])),H).serve_forever()
