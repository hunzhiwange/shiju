import json,sys
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
class Handler(BaseHTTPRequestHandler):
 def log_message(self,*args): pass
 def do_GET(self): self.send_error(405)
 def do_DELETE(self): self.send_response(200); self.end_headers()
 def do_POST(self):
  request=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
  method=request.get('method')
  if 'id' not in request:
   self.send_response(202); self.end_headers(); return
  if method=='initialize': result={'protocolVersion':'2024-11-05','capabilities':{'tools':{}},'serverInfo':{'name':'fixture','version':'1'}}
  elif method=='tools/list': result={'tools':[{'name':'echo','description':'Echo','inputSchema':{'type':'object'},'annotations':{'readOnlyHint':True}}]}
  elif method=='tools/call':
   with open(sys.argv[2],'a') as f: f.write(json.dumps(request['params'])+'\n')
   result={'content':[{'type':'text','text':json.dumps({'hookSpecificOutput':{'additionalContext':'fixture-context'}})}]}
  else: result={}
  body=json.dumps({'jsonrpc':'2.0','id':request['id'],'result':result}).encode()
  self.send_response(200); self.send_header('Content-Type','application/json'); self.send_header('Content-Length',str(len(body))); self.end_headers(); self.wfile.write(body)
server=ThreadingHTTPServer(('127.0.0.1',0),Handler)
with open(sys.argv[1],'w') as f: f.write(str(server.server_port))
server.serve_forever()
