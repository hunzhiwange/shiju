"""Real pinned Snow ACP; only the four model HTTP services are simulated."""
import json, os, secrets, selectors, subprocess, tempfile, threading, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path.cwd()
results = []
for method in ('chat', 'responses', 'anthropic', 'gemini'):
    key = secrets.token_hex(24)
    observed = []
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_args): pass
        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            authenticated = any(self.headers.get(h) == value for h, value in (
                ('Authorization', 'Bearer ' + key), ('x-api-key', key), ('x-goog-api-key', key)))
            model = body.get('model', 'probe-model' if '/models/probe-model:' in self.path else None)
            observed.append({'path': self.path, 'model': model, 'authenticated': authenticated})
            def event(kind, data): return ('event: '+kind+'\n' if kind else '')+'data: '+json.dumps(data)+'\n\n'
            if method == 'chat':
                payload = event('', {'id':'probe','object':'chat.completion.chunk','model':'probe-model','choices':[{'index':0,'delta':{'role':'assistant','content':'PROTOCOL_OK'},'finish_reason':None}]})
                payload += event('', {'id':'probe','object':'chat.completion.chunk','model':'probe-model','choices':[{'index':0,'delta':{},'finish_reason':'stop'}]})+'data: [DONE]\n\n'
            elif method == 'responses':
                item = {'id':'message-probe','type':'message','role':'assistant','status':'completed','content':[{'type':'output_text','text':'PROTOCOL_OK','annotations':[]}]}
                payload = event('response.created', {'type':'response.created','response':{'id':'response-probe','object':'response','status':'in_progress','model':'probe-model','output':[]}})
                payload += event('response.output_text.delta', {'type':'response.output_text.delta','item_id':'message-probe','output_index':0,'content_index':0,'delta':'PROTOCOL_OK'})
                payload += event('response.output_item.done', {'type':'response.output_item.done','output_index':0,'item':item})
                payload += event('response.completed', {'type':'response.completed','response':{'id':'response-probe','object':'response','status':'completed','model':'probe-model','output':[item],'usage':{'input_tokens':1,'output_tokens':1,'total_tokens':2}}})
            elif method == 'anthropic':
                payload = event('message_start', {'type':'message_start','message':{'id':'message-probe','type':'message','role':'assistant','content':[],'model':'probe-model','stop_reason':None,'stop_sequence':None,'usage':{'input_tokens':1,'output_tokens':0}}})
                payload += event('content_block_start', {'type':'content_block_start','index':0,'content_block':{'type':'text','text':''}})
                payload += event('content_block_delta', {'type':'content_block_delta','index':0,'delta':{'type':'text_delta','text':'PROTOCOL_OK'}})
                payload += event('content_block_stop', {'type':'content_block_stop','index':0})
                payload += event('message_delta', {'type':'message_delta','delta':{'stop_reason':'end_turn','stop_sequence':None},'usage':{'output_tokens':1}})
                payload += event('message_stop', {'type':'message_stop'})
            else:
                payload = event('', {'candidates':[{'index':0,'content':{'role':'model','parts':[{'text':'PROTOCOL_OK'}]},'finishReason':'STOP'}],'usageMetadata':{'promptTokenCount':1,'candidatesTokenCount':1,'totalTokenCount':2},'modelVersion':'probe-model'})
            data = payload.encode()
            self.send_response(200); self.send_header('Content-Type','text/event-stream'); self.send_header('Content-Length',str(len(data))); self.end_headers(); self.wfile.write(data)
    server = ThreadingHTTPServer(('127.0.0.1',0),Handler)
    thread = threading.Thread(target=server.serve_forever,daemon=True); thread.start()
    try:
        with tempfile.TemporaryDirectory(prefix='snow-protocol-') as tmp:
            root = Path(tmp); os.chmod(root,0o700)
            base = 'http://127.0.0.1:'+str(server.server_port)+('/v1beta' if method == 'gemini' else '/v1')
            config = root/'config.json'; config.write_text(json.dumps({'snowcfg':{'baseUrl':base,'baseUrlMode':'base','apiKey':key,'requestMethod':method,'advancedModel':'probe-model','basicModel':'probe-model','maxTokens':1024,'maxRetries':0}})); os.chmod(config,0o600)
            env = {k:v for k,v in os.environ.items() if k in ('PATH','HOME','TMPDIR')}; env['SNOW_CONFIG_DIR']=tmp
            process = subprocess.Popen(['node',str(ROOT/'target/snow-cli-0.9.14/node_modules/snow-ai/bundle/cli.mjs'),'--acp'],cwd=tmp,env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True)
            selector = selectors.DefaultSelector(); selector.register(process.stdout,selectors.EVENT_READ)
            text = []
            def rpc(ident, method_name, params):
                process.stdin.write(json.dumps({'jsonrpc':'2.0','id':ident,'method':method_name,'params':params})+'\n'); process.stdin.flush()
                deadline = time.monotonic()+25
                while time.monotonic()<deadline:
                    if not selector.select(timeout=1): continue
                    line = process.stdout.readline()
                    if not line: raise RuntimeError('CLI exited')
                    try: message = json.loads(line)
                    except ValueError: continue
                    if message.get('method') == 'session/update':
                        update = message['params']['update']
                        if update.get('sessionUpdate') == 'agent_message_chunk': text.append(update['content'].get('text',''))
                    if message.get('id') == ident and 'method' not in message: return message
                raise RuntimeError('ACP timeout')
            try:
                initialized = rpc(1,'initialize',{'protocolVersion':1,'clientCapabilities':{},'clientInfo':{'name':'snow-native-probe','version':'1'}})
                session = rpc(2,'session/new',{'cwd':tmp,'mcpServers':[]})['result']['sessionId']
                reply = rpc(3,'session/prompt',{'sessionId':session,'prompt':[{'type':'text','text':'Reply PROTOCOL_OK.'}]})
                expected_path = {'chat':'/v1/chat/completions','responses':'/v1/responses','anthropic':'/v1/messages','gemini':'/v1beta/models/probe-model:streamGenerateContent?alt=sse'}[method]
                passed = initialized.get('result',{}).get('agentInfo',{}).get('version') == '0.9.14' and 'result' in reply and 'PROTOCOL_OK' in ''.join(text) and len(observed)==1 and all(r['authenticated'] and r['model']=='probe-model' and r['path']==expected_path for r in observed)
                results.append({'requestMethod':method,'passed':passed,'requests':observed})
            finally:
                process.terminate()
                try: process.wait(timeout=5)
                except subprocess.TimeoutExpired: process.kill(); process.wait()
                selector.close(); process.stdin.close(); process.stdout.close()
    except Exception as error:
        results.append({'requestMethod':method,'passed':False,'reason':type(error).__name__,'requests':observed})
    finally:
        server.shutdown(); server.server_close(); thread.join()
(ROOT/'target/snow-native-protocol-results.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps(results))
raise SystemExit(0 if all(r['passed'] for r in results) and len(results)==4 else 1)
