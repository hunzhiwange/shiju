"""Exercise aionrs native MCP through its single sandbox-authorized TCP port."""
import json
import hmac
import os
from pathlib import Path
import queue
import re
import secrets
import signal
import subprocess
import sys
import tempfile
import threading
import time
from http.client import HTTPConnection
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

TOOL_CASES = [
    ('rustcodegraph_query', {'search': 'createRueSiteApp'}, 'createRueSiteApp'),
    ('rustcodegraph_callers', {'symbol': 'createRueSiteApp'}, 'mountRueSiteApp'),
    ('rustcodegraph_node', {'name': 'mountRueSiteApp'}, 'createRueSiteApp'),
]
MODEL = 'local-probe'


def run(case, engine, graph, workspace, policy_template, root):
    home = root / 'state'
    home.mkdir()
    write_root = root / 'writes'
    write_root.mkdir()
    for config in (home/'.aionrs.toml', home/'.config/aionrs/config.toml',
                   home/'Library/Application Support/aionrs/config.toml'):
        config.parent.mkdir(parents=True, exist_ok=True)
        config.write_text('[tools]\nauto_approve = false\nallow_list = []\n[session]\nenabled = false\n')
    token = secrets.token_hex(32)
    graph_process = None
    if case in ('allow', 'deny'):
        graph_process = subprocess.Popen(
            [graph, 'serve', '--mcp', '--path', workspace, '--no-watch'],
            cwd=workspace, env={'PATH':'/usr/bin:/bin', 'HOME':str(home)},
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            text=True, start_new_session=True,
        )
    lock = threading.Lock()
    grants, model_calls, actual_calls, frames = [], [], [], []
    unexpected = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def respond(self, status, body=b'', mime='application/json'):
            self.send_response(status)
            self.send_header('Content-Type', mime)
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_POST(self):
            length = int(self.headers['Content-Length'])
            authorization = self.headers.get('Authorization', '').encode()
            if not hmac.compare_digest(authorization, ('Bearer ' + token).encode()) or length > 4 * 1024 * 1024:
                self.respond(403)
                return
            request = json.loads(self.rfile.read(length))
            try:
                if self.path == '/mcp' and graph_process is not None:
                    with lock:
                        method = request['method']
                        if method == 'tools/call':
                            params = request['params']
                            if not grants or grants.pop(0) != (params['name'], params['arguments']):
                                self.respond(403)
                                return
                            actual_calls.append(params['name'])
                        elif method not in {'initialize', 'notifications/initialized', 'tools/list'}:
                            self.respond(403)
                            return
                        graph_process.stdin.write(json.dumps(request)+'\n')
                        graph_process.stdin.flush()
                        result = json.loads(graph_process.stdout.readline()) if 'id' in request else None
                    self.respond(200, json.dumps(result).encode()) if result is not None else self.respond(202)
                elif self.path == '/v1/chat/completions':
                    names = [tool['function']['name'] for tool in request.get('tools', [])]
                    active = [name for name, _, _ in TOOL_CASES if name in names]
                    assert len(active) == (3 if graph_process else 0), active
                    index = len(model_calls)
                    model_calls.append(request)
                    if index:
                        results = [m['content'] for m in request['messages'] if m['role']=='tool']
                        assert results and (TOOL_CASES[index-1][2] if case=='allow' else 'Tool denied') in results[-1], results
                    if graph_process and index < (3 if case=='allow' else 1):
                        name, args, _ = TOOL_CASES[index]
                        delta = {'tool_calls':[{'index':0,'id':f'probe_{index}','type':'function',
                                                'function':{'name':name,'arguments':json.dumps(args)}}]}
                        reason = 'tool_calls'
                    else:
                        delta = {'content':'Finished' if graph_process else 'MCP_NOT_AVAILABLE'}
                        reason = 'stop'
                    chunks = [{'id':'probe','object':'chat.completion.chunk','created':1,'model':MODEL,
                               'choices':[{'index':0,'delta':piece,'finish_reason':finish}]}
                              for piece,finish in [({'role':'assistant'},None),(delta,None),({},reason)]]
                    body = ''.join('data: '+json.dumps(c)+'\n\n' for c in chunks)+'data: [DONE]\n\n'
                    self.respond(200,body.encode(),'text/event-stream')
                else:
                    self.respond(404)
            except Exception as error:
                unexpected.append(str(error))
                self.respond(500)

    listener = ThreadingHTTPServer(('127.0.0.1',0), Handler)
    port = listener.server_port
    profile_fields = {key:json.dumps(str(value)) for key,value in {
        'workspace':workspace, 'state':home, 'write_root':write_root, 'engine':engine,
        'git':workspace/'.git', 'local_config':workspace/'.aionrs.toml',
    }.items()}
    policy = root/'engine.sb'
    policy.write_text(policy_template.format(port=port, **profile_fields))
    env = {'PATH':'/usr/bin:/bin','HOME':str(home),'USERPROFILE':str(home),
           'CFFIXED_USER_HOME':str(home),'XDG_CONFIG_HOME':str(home/'.config'),'TMPDIR':str(home),
           'NO_PROXY':'127.0.0.1,localhost','API_KEY':token,'MODEL':MODEL,'PROVIDER':'openai',
           'BASE_URL':f'http://127.0.0.1:{port}/v1'}
    threading.Thread(target=listener.serve_forever, daemon=True).start()
    process = None
    try:
        process = subprocess.Popen(
            ['/usr/bin/sandbox-exec','-f',policy,engine,'--json-stream','--no-color','--project-dir',home],
            cwd=workspace,env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,
            text=True,start_new_session=True,
        )
        pending = queue.Queue()
        def read_frames():
            for line in process.stdout:
                pending.put(json.loads(line))
            pending.put(None)
        threading.Thread(target=read_frames,daemon=True).start()

        deadline = time.monotonic() + 25

        def recv():
            packet = pending.get(timeout=max(0, deadline - time.monotonic()))
            if packet is None:
                raise RuntimeError('native engine closed stdout')
            frames.append(packet)
            return packet

        def send(packet):
            process.stdin.write(json.dumps(packet)+'\n')
            process.stdin.flush()

        ready = recv()
        assert ready['type']=='ready' and ready['version']=='0.2.11', ready
        assert ready['capabilities']['tool_approval'] and ready['capabilities']['current_mode']=='default'
        if graph_process:
            send({'type':'add_mcp_server','name':'ruejs_graph_probe','transport':'streamable-http',
                  'url':f'http://127.0.0.1:{port}/mcp','headers':{'Authorization':'Bearer '+token}})
            mcp = recv()
            assert mcp['type']=='mcp_ready', mcp
            assert {name for name, _, _ in TOOL_CASES}.issubset(mcp['tools'])
        send({'type':'message','msg_id':'probe','content':'Run the tool round-trip probe.'})
        while True:
            packet = recv()
            kind = packet['type']
            if kind=='tool_request':
                tool = packet['tool']
                assert tool['category']=='mcp' and packet['msg_id']=='probe', packet
                if case=='allow':
                    with lock:
                        grants.append((tool['name'],tool['args']))
                    send({'type':'tool_approve','call_id':packet['call_id'],'scope':'once'})
                else:
                    assert case=='deny'
                    send({'type':'tool_deny','call_id':packet['call_id'],'reason':'Probe denied'})
            elif kind=='stream_end':
                assert packet['msg_id']=='probe'
                break
            elif kind=='error':
                raise AssertionError(packet)
        # A direct call to the broker must fail even with the current token
        # after the corresponding native approval has been consumed.
        name, args, _ = TOOL_CASES[0]
        replay = json.dumps({'jsonrpc': '2.0', 'id': 100, 'method': 'tools/call',
                             'params': {'name': name, 'arguments': args}})
        connection = HTTPConnection('127.0.0.1', port, timeout=5)
        try:
            for bearer, expected in [('invalid', 403), (token, 403 if graph_process else 404)]:
                connection.request('POST', '/mcp', replay, {'Authorization': 'Bearer ' + bearer})
                response = connection.getresponse()
                assert response.status == expected, response.status
                response.read()
        finally:
            connection.close()
        assert not unexpected, unexpected
        assert actual_calls == ([name for name, _, _ in TOOL_CASES] if case=='allow' else [])
        kinds = [frame['type'] for frame in frames]
        assert kinds.count('tool_request') == {'allow':3,'deny':1}.get(case,0)
        assert kinds.count('tool_cancelled') == (1 if case=='deny' else 0)
        success = [f for f in frames if f['type']=='tool_result' and f['status']=='success']
        assert len(success) == (3 if case=='allow' else 0), frames
        print(case,'model_rounds',len(model_calls),'mcp_calls',actual_calls,flush=True)
    finally:
        if process is not None and process.poll() is None:
            os.killpg(process.pid,signal.SIGTERM)
            process.wait(timeout=5)
        if graph_process is not None and graph_process.poll() is None:
            os.killpg(graph_process.pid,signal.SIGTERM)
            graph_process.wait(timeout=5)
        listener.shutdown()
        listener.server_close()


def main():
    if len(sys.argv) != 4:
        raise SystemExit(f'usage: {sys.argv[0]} ENGINE RUSTCODEGRAPH INDEXED_WORKSPACE')
    engine, graph, workspace = (Path(arg).resolve(strict=True) for arg in sys.argv[1:])
    repo = Path(__file__).resolve().parents[3]
    driver = repo / 'crates/libs/llm-runtime/src/acp/model_source/drivers/aionrs.rs'
    literal = re.search(r'let policy = format!\(\s*"((?:\\.|[^"\\])*)"',driver.read_text(),re.S)[1]
    policy_template = re.sub(r'\\\n\s*','',literal).replace('\\n','\n').replace('\\"','"')
    with tempfile.TemporaryDirectory(prefix='aionrs-proof-') as work:
        for case in ['unmounted','allow','deny','removed']:
            root=Path(work).resolve()/case
            root.mkdir()
            run(case,engine,graph,workspace,policy_template,root)


if __name__=='__main__':
    main()
