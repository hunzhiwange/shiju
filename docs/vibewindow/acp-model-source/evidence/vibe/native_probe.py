"""Real patched Mistral Vibe 3.0.4 ACP; only the four model HTTP services are simulated."""
import json, os, secrets, selectors, subprocess, tempfile, threading, time, sys, queue, argparse, platform
import tomli_w
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path.cwd()
parser = argparse.ArgumentParser()
parser.add_argument('--mode', choices=('session-model', 'cli-local'), default='session-model')
args = parser.parse_args()
results = []
for method in ('chat', 'responses', 'anthropic'):
    key = secrets.token_hex(24)
    observed = []
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_args): pass
        def do_POST(self):
            raw = self.rfile.read(int(self.headers.get('Content-Length', '0')))
            if not raw:
                self.send_response(404); self.end_headers(); return
            body = json.loads(raw)
            authenticated = any(self.headers.get(h) == value for h, value in (
                ('Authorization', 'Bearer ' + key), ('x-api-key', key), ('x-goog-api-key', key)))
            model = body.get('model', 'probe-model' if '/models/probe-model:' in self.path else None)
            observed.append({'path': self.path, 'model': model, 'authenticated': authenticated})
            def event(kind, data): return ('event: '+kind+'\n' if kind else '')+'data: '+json.dumps(data)+'\n\n'
            if method == 'chat':
                payload = event('', {'id':'probe','object':'chat.completion.chunk','model':'probe-model','choices':[{'index':0,'delta':{'role':'assistant','content':'PROTOCOL_OK'},'finish_reason':None}]})
                payload += event('', {'id':'probe','object':'chat.completion.chunk','model':'probe-model','choices':[{'index':0,'delta':{},'finish_reason':'stop'}], 'usage':{'prompt_tokens':1,'completion_tokens':1,'total_tokens':2}})+'data: [DONE]\n\n'
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
        with tempfile.TemporaryDirectory(prefix='vibe-protocol-') as tmp:
            root = Path(tmp); os.chmod(root,0o700)
            base = 'http://127.0.0.1:'+str(server.server_port)+('' if method == 'anthropic' else '/v1')
            model = {'name':'probe-model','alias':'session-selected','provider':'session-provider', 'max_output_tokens':1024}
            settings = {'active_model':'session-selected','allowed_models':['probe-model'],
                        'providers':[{'name':'session-provider','api_base':base,'api_key_env_var':'VW_VIBE_RELAY_TOKEN',
                                      'api_style':{'chat':'openai','responses':'openai-responses','anthropic':'anthropic'}[method]}],
                        'models':[model],'compaction_model':model,
                        'utility_models':{'title':'session-selected','smart_approve':'session-selected'}}
            config = root/'session-model.toml';config.write_text(tomli_w.dumps(settings));config.chmod(0o600)
            (root/'config.toml').write_text('enable_telemetry = false\n' + (tomli_w.dumps(settings) if args.mode == 'cli-local' else ''))
            env = {k:v for k,v in os.environ.items() if k in ('PATH','HOME','TMPDIR')}
            env.update(VIBE_HOME=tmp,VW_VIBE_RELAY_TOKEN=key)
            launch = [str(ROOT/'target/vibe-cli-env/bin/vibe-acp')]
            if args.mode == 'session-model': launch += ['--session-model-config',str(config)]
            process = subprocess.Popen(launch,cwd=tmp,env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True)
            messages = queue.Queue()
            def read_messages():
                for line in process.stdout:
                    messages.put(line)
                messages.put(None)
            reader = threading.Thread(target=read_messages, daemon=True); reader.start()
            text = []
            def rpc(ident, method_name, params):
                process.stdin.write(json.dumps({'jsonrpc':'2.0','id':ident,'method':method_name,'params':params})+'\n'); process.stdin.flush()
                deadline = time.monotonic()+25
                while time.monotonic()<deadline:
                    try: line = messages.get(timeout=1)
                    except queue.Empty: continue
                    if not line: raise RuntimeError('CLI exited')
                    try: message = json.loads(line)
                    except ValueError: continue
                    if message.get('method') == 'session/update':
                        update = message['params']['update']
                        if update.get('sessionUpdate') == 'agent_message_chunk': text.append(update['content'].get('text',''))
                    if message.get('id') == ident and 'method' not in message: return message
                raise RuntimeError('ACP timeout')
            try:
                initialized = rpc(1,'initialize',{'protocolVersion':1,'clientCapabilities':{},'clientInfo':{'name':'vibe-native-probe','version':'1'}})
                session = rpc(2,'session/new',{'cwd':tmp,'mcpServers':[]})['result']['sessionId']
                reply = rpc(3,'session/prompt',{'sessionId':session,'prompt':[{'type':'text','text':'Reply PROTOCOL_OK.'}]})
                expected_path = {'chat':'/v1/chat/completions','responses':'/v1/responses','anthropic':'/v1/messages','gemini':'/v1beta/models/probe-model:streamGenerateContent?alt=sse'}[method]
                passed = initialized.get('result',{}).get('agentInfo',{}).get('version') == '3.0.4' and 'result' in reply and 'PROTOCOL_OK' in ''.join(text) and len(observed)>=1 and all(r['authenticated'] and r['model']=='probe-model' and r['path']==expected_path for r in observed)
                results.append({'requestMethod':method,'passed':passed,'requests':observed, 'reply':reply if not passed else None, 'text':''.join(text) if not passed else None})
            finally:
                process.terminate()
                try: process.wait(timeout=5)
                except subprocess.TimeoutExpired: process.kill(); process.wait()
                reader.join(timeout=5); process.stdin.close(); process.stdout.close()
    except Exception as error:
        results.append({'requestMethod':method,'passed':False,'reason':type(error).__name__,'requests':observed})
    finally:
        server.shutdown(); server.server_close(); thread.join()
for record in results:
    record.update(version='3.0.4', platform=platform.system()+'-'+platform.machine(), mode=args.mode)
(ROOT/('target/vibe-native-'+args.mode+'-results.json')).write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps(results))
raise SystemExit(0 if all(r['passed'] for r in results) and len(results)==3 else 1)
