# VibeWindow-authored fixture. MIT, matching the repository license.
import http.server,json,pathlib,subprocess,sys
mode,journal=sys.argv[1:3]
class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version='HTTP/1.1'
    def log_message(self,*args):pass
    def do_GET(self):
        self.send_response(405);self.send_header('Content-Length','0');self.end_headers()
    def do_POST(self):
        request=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        if 'id' not in request:
            self.send_response(202);self.send_header('Content-Length','0');self.end_headers();return
        result=subprocess.run([sys.executable,str(pathlib.Path(__file__).with_name('server.py')),mode,journal],input=json.dumps(request)+'\n',capture_output=True,text=True,check=True)
        response=result.stdout.strip()
        payload=(('event: message\ndata: '+response+'\n\n') if mode=='http-sse' else response).encode()
        self.send_response(200);self.send_header('Content-Type','text/event-stream' if mode=='http-sse' else 'application/json');self.send_header('Content-Length',str(len(payload)));self.end_headers();self.wfile.write(payload);self.wfile.flush()
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
print(server.server_port,flush=True)
server.serve_forever()
