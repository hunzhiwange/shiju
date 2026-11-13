"""Real reviewed CLI with a credential-free loopback model; no user configuration is read."""
import json,os,subprocess,tempfile,threading,http.server,queue,sys
from pathlib import Path
entry = Path(sys.argv[1]).resolve()
gateway = len(sys.argv)>2 and sys.argv[2]=="--gateway"
root=tempfile.mkdtemp(prefix='deepagents-real-')
requests=[]
cancel_started=threading.Event()
cancel_release=threading.Event()
cancel_phase=False
tool_name='write_file'
class Handler(http.server.BaseHTTPRequestHandler):
 def log_message(self,*a):pass
 def do_POST(self):
  r=json.loads(self.rfile.read(int(self.headers['Content-Length']))); requests.append(r)
  if self.headers.get('Authorization') != 'Bearer local-fixture' or r.get('model') != 'gpt-4o':
   self.send_response(401 if self.headers.get('Authorization') != 'Bearer local-fixture' else 404)
   self.send_header('Content-Type','application/json');self.end_headers()
   self.wfile.write(json.dumps({'error':{'message':'fixture rejected','type':'invalid_request_error','code':'fixture'}}).encode());return
  if cancel_phase:
   cancel_started.set();cancel_release.wait(timeout=10)
  # First call of each round asks to write a real temporary file.
  tools=r.get('tools',[])
  tool=next((t for t in tools if t['function']['name']==tool_name),None)
  message={'role':'assistant','content':'验收回复'}
  if tool and r.get('messages', [{}])[-1].get('role') != 'tool' and not gateway and not cancel_phase and (len(requests)<13 or tool_name=='edit_file'):
   schema=tool['function']['parameters']
   message={'role':'assistant','content':'','tool_calls':[{'id':'call_fixture_'+str(len(requests)),'type':'function','function':{'name':tool_name,'arguments':json.dumps({'file_path':root+'/result.txt','content':'fixture'} if tool_name=='write_file' else {'file_path':root+'/result.txt','old_string':'fixture','new_string':'bypass'})}}]}
  self.send_response(200)
  if r.get('stream'):
   self.send_header('Content-Type','text/event-stream');self.end_headers()
   delta=dict(message);delta.pop('role',None)
   for i,t in enumerate(delta.get('tool_calls',[])):t['index']=i
   chunks=[{'id':'fixture_'+str(len(requests)),'object':'chat.completion.chunk','model':'fixture','choices':[{'index':0,'delta':delta,'finish_reason':None}]},{'id':'fixture_'+str(len(requests)),'object':'chat.completion.chunk','model':'fixture','choices':[{'index':0,'delta':{},'finish_reason':'tool_calls' if message.get('tool_calls') else 'stop'}]}]
   for chunk in chunks:self.wfile.write(('data: '+json.dumps(chunk)+'\n\n').encode())
   self.wfile.write(b'data: [DONE]\n\n')
  else:
   self.send_header('Content-Type','application/json');self.end_headers(); self.wfile.write(json.dumps({'id':'fixture_'+str(len(requests)),'object':'chat.completion','model':'fixture','choices':[{'index':0,'message':message,'finish_reason':'tool_calls' if message.get('tool_calls') else 'stop'}]}).encode())
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler);threading.Thread(target=server.serve_forever,daemon=True).start()
inherited = {key:value for key,value in os.environ.items() if key in {
 'PATH','TMPDIR','TEMP','TMP','LANG','LC_ALL','SYSTEMROOT','SystemRoot',
 'CARGO_TARGET_DIR','CARGO_BUILD_BUILD_DIR','SHIJU_CARGO_SLOT_TOKEN','SHIJU_CARGO_SLOT_ACTIVE','VW_DEEPAGENTS_WINDOW_ROOT','VIBEWINDOW_CONFIG_DIR'}}
inherited['RUSTUP_HOME']=os.environ.get('RUSTUP_HOME',str(Path.home()/'.rustup'))
inherited['CARGO_HOME']=os.environ.get('CARGO_HOME',str(Path.home()/'.cargo'))
env=dict(inherited,HOME=root,OPENAI_API_KEY='local-fixture',OPENAI_BASE_URL=f'http://127.0.0.1:{server.server_port}/v1',LANGSMITH_TRACING='false')
if gateway:
 try:
  result=subprocess.run(sys.argv[3:],env=dict(env,HOME=os.environ['HOME'],SHIJU_DEEPAGENTS_TEST_ENTRY=str(entry),SHIJU_DEEPAGENTS_TEST_ENDPOINT=env['OPENAI_BASE_URL']))
  if result.returncode == 0:
   assert requests and all(r['model']=='gpt-4o' for r in requests)
   print('GATEWAY_MODEL_REQUESTS',len(requests),'MODEL gpt-4o',flush=True)
  raise SystemExit(result.returncode)
 finally:server.shutdown()
p=subprocess.Popen(['node',str(entry),'--model','openai:gpt-4o','--workspace',root],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=open(root+'/stderr','w'),text=True,env=env)
q=queue.Queue()
def reader():
 for line in p.stdout:q.put(json.loads(line))
threading.Thread(target=reader,daemon=True).start()
permission='reject-once'
write_allowed=False
permissions=0
write_attempts=0
sid=None
def send(r):p.stdin.write(json.dumps(dict(jsonrpc='2.0',**r))+'\n');p.stdin.flush()
def rpc(i,m,params):
 global permissions,write_attempts
 send(dict(id=i,method=m,params=params))
 while True:
  r=q.get(timeout=45)
  if 'method' in r and 'id' in r:
   pass
   if r['method']=='session/request_permission':
    permissions+=1
    if permission=='rpc-error':
     send({'id':r['id'],'error':{'code':-32000,'message':'fixture permission failure'}});continue
    value={'outcome':{'outcome':'cancelled'} if permission=='cancelled' else {'outcome':'selected','optionId':permission}}
   elif r['method']=='fs/write_text_file':
    write_attempts+=1
    if not write_allowed:
     send({'id':r['id'],'error':{'code':-32000,'message':'host denied'}});continue
    Path(r['params']['path']).write_text(r['params']['content']);value={}
   elif r['method']=='fs/read_text_file':value={'content':''}
   else:value={}
   send({'id':r['id'],'result':value})
  if r.get('id')==i:return r
try:
 print('INIT',rpc(1,'initialize',{'protocolVersion':1,'clientCapabilities':{'fs':{'readTextFile':True,'writeTextFile':True}},'clientInfo':{'name':'fixture','version':'1'}}),flush=True)
 new=rpc(2,'session/new',{'cwd':root,'mcpServers':[]});print('NEW',new,flush=True);sid=new['result']['sessionId']
 assert 'result' in rpc(92,'session/set_mode',{'sessionId':sid,'modeId':'plan'})
 assert rpc(93,'session/prompt',{'sessionId':sid,'prompt':[{'type':'text','text':'/status'}]})['result']['stopReason']=='end_turn'
 assert 'result' in rpc(94,'session/set_mode',{'sessionId':sid,'modeId':'agent'})
 print('Mode switches and status command passed',flush=True)
 for i,permission in enumerate(['reject-once','rpc-error','cancelled','unknown','allow-once','allow-once'],3):
  write_allowed=i==8
  result=rpc(i,'session/prompt',{'sessionId':sid,'prompt':[{'type':'text','text':'fixture'}]})
  assert ('error' in result) == (i==7), result
  assert os.path.exists(root+'/result.txt') == write_allowed
  print('ROUND',i,'permission',permission,'host_write',write_allowed,'passed',flush=True)
 assert permissions==6 and write_attempts==2,(permissions,write_attempts)
 assert Path(root+'/result.txt').read_text()=='fixture'
 tool_name='edit_file'
 before_permissions=permissions
 assert 'result' in rpc(95,'session/prompt',{'sessionId':sid,'prompt':[{'type':'text','text':'fixture edit denial'}]})
 assert permissions==before_permissions and Path(root+'/result.txt').read_text()=='fixture'
 print('Inherited local edit backend rejected before execution',flush=True)
 tool_name='write_file'
 assert 'error' in rpc(90,'session/load',{'sessionId':sid,'cwd':root,'mcpServers':[{'name':'fixture','command':'false','args':[],'env':[]}]})
 assert 'error' in rpc(91,'session/load',{'sessionId':sid,'cwd':root+'/other','mcpServers':[]})
 print('MCP and workspace expansion rejected',flush=True)
 assert 'result' in rpc(9,'session/load',{'sessionId':sid,'cwd':root,'mcpServers':[]})
 print('LOAD same session passed',flush=True)
 cancel_phase=True
 send({'id':10,'method':'session/prompt','params':{'sessionId':sid,'prompt':[{'type':'text','text':'fixture cancel'}]}})
 assert cancel_started.wait(timeout=10)
 send({'method':'session/cancel','params':{'sessionId':sid}})
 cancel_release.set()
 while True:
  result=q.get(timeout=15)
  if result.get('id')==10:
   assert result.get('result',{}).get('stopReason')=='cancelled',result
   break
 cancel_phase=False
 result=rpc(11,'session/prompt',{'sessionId':sid,'prompt':[{'type':'text','text':'fixture after cancel'}]})
 assert 'result' in result,result
 print('CANCEL and continuation passed',flush=True)
 print('MODELS',set(r['model'] for r in requests),flush=True)
 for model,key in [('openai:gpt-4o','invalid-fixture'),('openai:invalid-model','local-fixture')]:
  p.terminate();p.wait(timeout=10)
  q=queue.Queue()
  failed_env=dict(env,OPENAI_API_KEY=key)
  p=subprocess.Popen(['node',str(entry),'--model',model,'--workspace',root],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=open(root+'/negative-stderr','w'),text=True,env=failed_env)
  threading.Thread(target=reader,daemon=True).start()
  assert 'result' in rpc(20,'initialize',{'protocolVersion':1,'clientCapabilities':{'fs':{'readTextFile':True,'writeTextFile':True}}})
  new=rpc(21,'session/new',{'cwd':root,'mcpServers':[]});sid=new['result']['sessionId']
  before=len(requests)
  result=rpc(22,'session/prompt',{'sessionId':sid,'prompt':[{'type':'text','text':'fixture'}]})
  assert 'error' in result,result
  assert len(requests)>before and all(r['model']==model.split(':',1)[1] for r in requests[before:])
  print('AUTH or MODEL rejection without fallback passed',flush=True)
finally:
 p.terminate();p.wait(timeout=10);server.shutdown();print('ROOT',root,flush=True)
