# SPDX-License-Identifier: MIT
# Local MCP 2025-06-18 form fixture; synthetic data only.
import json
import sys

def send(v):
    print(json.dumps(v), flush=True)

pending = None
for line in sys.stdin:
    v = json.loads(line)
    method = v.get('method')
    if method == 'initialize':
        send({'jsonrpc':'2.0','id':v['id'],'result':{'protocolVersion':'2025-06-18','capabilities':{'tools':{}},'serverInfo':{'name':'form-fixture','version':'1'}}})
    elif method == 'tools/list':
        send({'jsonrpc':'2.0','id':v['id'],'result':{'tools':[{'name':'form','inputSchema':{'type':'object'}}]}})
    elif method == 'tools/call':
        pending = v['id']
        send({'jsonrpc':'2.0','id':91,'method':'elicitation/create','params':{'message':'Synthetic form','requestedSchema':{'type':'object','properties':{'age':{'type':'integer','minimum':18,'maximum':99},'choice':{'type':'string','enum':['a','b']}},'required':['age','choice']}}})
    elif v.get('id') == 91:
        send({'jsonrpc':'2.0','id':pending,'result':{'content':[{'type':'text','text':json.dumps(v)}]}})
