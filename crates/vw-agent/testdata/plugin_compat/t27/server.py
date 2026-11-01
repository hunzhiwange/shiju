# SPDX-License-Identifier: MIT
# Synthetic URL elicitation over a real stdio MCP peer; no network or credentials.
import json
import sys

def send(value):
    print(json.dumps(value), flush=True)

pending = None
for line in sys.stdin:
    value = json.loads(line)
    method = value.get('method')
    if method == 'initialize':
        send({'jsonrpc':'2.0','id':value['id'],'result':{'protocolVersion':'2025-06-18','capabilities':{'tools':{}},'serverInfo':{'name':'url-fixture','version':'1'}}})
    elif method == 'tools/list':
        send({'jsonrpc':'2.0','id':value['id'],'result':{'tools':[{'name':'authorize','inputSchema':{'type':'object'}}]}})
    elif method == 'tools/call':
        pending = value['id']
        send({'jsonrpc':'2.0','id':91,'method':'elicitation/create','params':{'mode':'url','message':'Authorize fixture','url':'https://auth.example/authorize','elicitationId':'url-completion-91'}})
    elif value.get('id') == 91:
        send({'jsonrpc':'2.0','id':pending,'result':{'content':[{'type':'text','text':json.dumps(value)}]}})
