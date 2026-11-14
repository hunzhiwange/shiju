import http.server,threading,json
requests=[]
tool_sent=False
class H(http.server.BaseHTTPRequestHandler):
 def log_message(self,*a):pass
 def do_POST(self):
  global tool_sent
  a=json.loads(self.rfile.read(int(self.headers['Content-Length'])));
  if 'slow fixture' in json.dumps(a.get('messages',[])): __import__('time').sleep(2)
  requests.append({'path':self.path,'model':a.get('model'),'stream':a.get('stream')})
  contact=__import__('os').environ.get('SHIJU_NOVA_TEST_CONTACT')
  if contact:
   with open(contact,'a') as capture:capture.write(str(a.get('model'))+'\n')
  status=int(__import__('os').environ.get('SHIJU_NOVA_TEST_HTTP_STATUS','200'))
  if status != 200:
   out=json.dumps({'type':'error','error':{'type':'authentication_error' if status==401 else 'not_found_error','message':'controlled fixture error'}}).encode()
   self.send_response(status);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(out)));self.end_headers();self.wfile.write(out);return
  tool=not tool_sent and 'controlled fixture' in json.dumps(a.get('messages',[]));tool_sent=tool_sent or tool; assert a.get('model') == __import__('os').environ.get('SHIJU_NOVA_TEST_EXPECTED_MODEL','claude-haiku-5-5'), a.get('model'); b={'id':'msg_fixture','type':'message','role':'assistant','model':a.get('model'),'content':[{'type':'text','text':'验收回复 fixture response'}],'stop_reason':'end_turn','stop_sequence':None,'usage':{'input_tokens':1,'output_tokens':1}}
  if tool:
   if __import__('os').environ.get('SHIJU_NOVA_TEST_TOOL')=='command':
    b['content']=[{'type':'tool_use','id':'tool_fixture','name':'execute_command','input':{'command':__import__('os').environ['SHIJU_NOVA_TEST_COMMAND'],'description':'Controlled marker command'}}]
   else:b['content']=[{'type':'tool_use','id':'tool_fixture','name':'edit_file','input':{'path':'fixture.txt','edits':[{'oldText':'before','newText':'after'}]}}]
   b['stop_reason']='tool_use'
  if a.get('stream'):
   items=[('message_start',{'type':'message_start','message':dict(b,content=[],stop_reason=None)}),('content_block_start',{'type':'content_block_start','index':0,'content_block':dict(b['content'][0],input={}) if tool else {'type':'text','text':''}}),('content_block_delta',{'type':'content_block_delta','index':0,'delta':{'type':'input_json_delta','partial_json':json.dumps(b['content'][0]['input'])} if tool else {'type':'text_delta','text':'验收回复 fixture response'}}),('content_block_stop',{'type':'content_block_stop','index':0}),('message_delta',{'type':'message_delta','delta':{'stop_reason':b['stop_reason'],'stop_sequence':None},'usage':{'output_tokens':1}}),('message_stop',{'type':'message_stop'})]
   out=''.join('event: '+k+'\ndata: '+json.dumps(v)+'\n\n' for k,v in items).encode(); ct='text/event-stream'
  else:out=json.dumps(b).encode();ct='application/json'
  self.send_response(200);self.send_header('Content-Type',ct);self.send_header('Content-Length',str(len(out)));self.end_headers();self.wfile.write(out)
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),H);threading.Thread(target=server.serve_forever,daemon=True).start()

import os,sys,subprocess
node=os.environ['SHIJU_NOVA_TEST_ENTRY']
env=dict(PATH=os.environ['PATH'],HOME=os.environ['HOME'],NO_PROXY='127.0.0.1,localhost',no_proxy='127.0.0.1,localhost',COMPASS_ANTHROPIC_BASE_URL='http://127.0.0.1:'+str(server.server_port),ANTHROPIC_API_KEY='fixture-key',COMPASS_TELEMETRY='false',COMPASS_ERROR_REPORTING='false',NODE_ENV='production',NODE_OPTIONS='--no-addons',COMPASS_API_KEY='')
try:
 capture=os.environ.get('SHIJU_NOVA_TEST_CAPTURE')
 if capture:
  child=subprocess.Popen(['node',node,'acp'],env=env,stdin=subprocess.PIPE,stdout=sys.stdout,stderr=sys.stderr,text=True)
  def forward():
   try:
    for line in sys.stdin:
     with open(capture,'a') as frames:frames.write(line)
     child.stdin.write(line);child.stdin.flush()
   finally:child.stdin.close()
  threading.Thread(target=forward,daemon=True).start()
  code=child.wait()
 else:
  code=subprocess.run(['node',node,'acp'],env=env,stdin=sys.stdin,stdout=sys.stdout,stderr=sys.stderr).returncode
finally:server.shutdown()
sys.exit(code)
