#!/usr/bin/env python3
"""Real Vibe tool rounds/cancellation/restart/switch against the task-10 HTTP fixture."""
import argparse
import hashlib
import shutil
import json
import os
from pathlib import Path
import queue
import platform
import subprocess
import sys
import tempfile
import threading
import time
import tomli_w

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / 'scripts'))
from acp_model_source_fixture import ModelFixture

parser = argparse.ArgumentParser()
parser.add_argument('--mode', choices=('session-model', 'cli-local'), required=True)
args = parser.parse_args()

class Client:
    def __init__(self, root, env, binding):
        command = [str(ROOT / 'target/vibe-cli-env/bin/vibe-acp')]
        if args.mode == 'session-model': command += ['--session-model-config', str(binding)]
        self.process = subprocess.Popen(command, cwd=root, env=env, text=True, stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.queue = queue.Queue()
        def reader():
            for line in self.process.stdout:
                self.queue.put(json.loads(line))
            self.queue.put(None)
        self.reader = threading.Thread(target=reader, daemon=True); self.reader.start()
        self.ident = 0

    def send(self, method, params, notification=False):
        self.ident += 1
        message = {'jsonrpc':'2.0','method':method,'params':params}
        if not notification: message['id'] = self.ident
        self.process.stdin.write(json.dumps(message)+'\n'); self.process.stdin.flush()
        return self.ident

    def wait(self, ident):
        deadline = time.monotonic()+40
        text = []
        while time.monotonic() < deadline:
            try: message = self.queue.get(timeout=1)
            except queue.Empty: continue
            if message is None: raise RuntimeError('real CLI exited')
            if message.get('method') == 'session/update':
                update = message['params']['update']
                if update.get('sessionUpdate') == 'agent_message_chunk':
                    text.append(update['content'].get('text',''))
            if message.get('id') == ident and 'method' not in message:
                if 'error' in message: raise RuntimeError(str(message['error']))
                return message['result'], ''.join(text)
        raise RuntimeError('ACP response timed out')

    def rpc(self, method, params): return self.wait(self.send(method, params))[0]

    def close(self):
        self.process.terminate()
        try: self.process.wait(timeout=5)
        except subprocess.TimeoutExpired: self.process.kill(); self.process.wait()
        self.reader.join(timeout=5)
        self.process.stdin.close(); self.process.stdout.close()

with tempfile.TemporaryDirectory(prefix='vibe-lifecycle-') as temporary, ModelFixture() as fixture:
    root = Path(temporary); root.chmod(0o700)
    home=root/'native';home.mkdir(mode=0o700)
    read=root/'challenge.txt';read.write_text(fixture.challenge);fixture.read_path=str(read)
    env={key:os.environ[key] for key in ('PATH','HOME','TMPDIR') if key in os.environ}
    env.update(VIBE_HOME=str(home),VW_VIBE_RELAY_TOKEN=fixture.key)
    binding=home/'session-model.toml'
    reference=hashlib.sha256(str(root).encode()).hexdigest()
    if args.mode=='session-model':
        env['VW_ACP_LAUNCH_REFERENCE']='vw-session-model:'+reference
    history=Path(tempfile.gettempdir())/f'shiju-vibe-state-{os.getuid()}'/reference
    def settings(model):
        route={'name':model,'alias':'session-selected','provider':'session-provider','max_output_tokens':1024}
        return {'active_model':'session-selected','allowed_models':[model],
                'models':[route],'compaction_model':route,
                'providers':[{'name':'session-provider','api_base':fixture.url+'/v1','api_key_env_var':'VW_VIBE_RELAY_TOKEN','api_style':'openai'}],
                'utility_models':{'title':'session-selected','smart_approve':'session-selected'}}
    def write(model):
        binding.write_text(tomli_w.dumps(settings(model)));binding.chmod(0o600)
        native=settings(model if args.mode=='cli-local' else 'forbidden-native-model')
        native['enable_telemetry']=False
        (home/'config.toml').write_text(tomli_w.dumps(native))
    def connect():
        client=Client(root,env,binding)
        initialized=client.rpc('initialize',{'protocolVersion':1,'clientCapabilities':{},'clientInfo':{'name':'vibe-lifecycle-probe','version':'1'}})
        assert initialized['agentInfo']['version']=='3.0.4'
        return client
    flow={}
    write(fixture.models[0]);client=connect()
    try:
        session=client.rpc('session/new',{'cwd':str(root),'mcpServers':[]})['sessionId'];flow['new']=True
        for label in ('first','second'):
            reply,text=client.wait(client.send('session/prompt',{'sessionId':session,'prompt':[{'type':'text','text':'Read challenge.txt twice using your read tool, then reply.'}]}))
            assert 'FIXTURE_MODEL_REPLY' in text and fixture.challenge in text
            flow[label]=True
        before=len(fixture.snapshot()['requests'])
        ident=client.send('session/prompt',{'sessionId':session,'prompt':[{'type':'text','text':'CANCEL_PROBE'}]})
        for _ in range(200):
            if len(fixture.snapshot()['requests'])>before: break
            time.sleep(.025)
        else: raise RuntimeError('cancellation did not reach model')
        client.send('session/cancel',{'sessionId':session},notification=True)
        reply,_=client.wait(ident);flow['cancel']=reply.get('stopReason')=='cancelled'
    finally:client.close()
    if args.mode=='session-model':
        home=root/'rotated-native';home.mkdir(mode=0o700)
        env['VIBE_HOME']=str(home);binding=home/'session-model.toml'
        write(fixture.models[1])
    client=connect()
    try:
        client.rpc('session/load',{'sessionId':session,'cwd':str(root),'mcpServers':[]});flow['restart']=True
        reply,text=client.wait(client.send('session/prompt',{'sessionId':session,'prompt':[{'type':'text','text':'Read challenge.txt twice using your read tool, then reply.'}]}))
        assert 'FIXTURE_MODEL_REPLY' in text and fixture.challenge in text
        flow['switch']=True
    finally:client.close()
    evidence=fixture.snapshot()
    token_absent=True
    if args.mode=='session-model':
        assert history.is_dir() and history.stat().st_mode & 0o777 == 0o700
        token_absent=all(fixture.key.encode() not in path.read_bytes() for path in history.rglob('*') if path.is_file() and not path.is_symlink())
    passed=token_absent and all(flow.values()) and evidence['errors']==0 and any(r['tool_results']>=2 for r in evidence['requests'])
    report={'mode':args.mode,'evidence_kind':'real-vibe-direct-acp','version':'3.0.4','platform':platform.system()+'-'+platform.machine(),'passed':passed,'token_persisted':not token_absent,'rotated_private_home':args.mode=='session-model','flow':flow,'requests':evidence}
    destination=ROOT/f'target/vibe-lifecycle-{args.mode}.json';destination.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report))
    if args.mode=='session-model':
        # Vibe materializes immutable plugin package directories in its store.
        # Only make this probe's own directories writable; never chmod files
        # that could be hard-linked to the supplier's package cache.
        for path in [history, *history.rglob('*')]:
            if path.is_dir() and not path.is_symlink():path.chmod(0o700)
        shutil.rmtree(history)
    raise SystemExit(0 if passed else 1)
