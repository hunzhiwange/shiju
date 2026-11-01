# Self-contained local MCP server; repository license. No network or credentials.
import json, pathlib, sys
for line in sys.stdin:
    r=json.loads(line)
    if 'id' not in r: continue
    m=r['method']
    if m=='initialize': result={'protocolVersion':'2025-06-18','capabilities':{'tools':{},'resources':{}},'serverInfo':{'name':'panel-qa','version':'1'}}
    elif m=='tools/list': result={'tools':[{'name':'show','inputSchema':{'type':'object'},'_meta':{'ui':{'resourceUri':'ui://panel/view'}}}]}
    elif m=='tools/call': result={'content':[{'type':'text','text':'本地服务结果'}]}
    elif m=='resources/read': result={'contents':[{'uri':'ui://panel/view','mimeType':'text/html;profile=mcp-app','text':pathlib.Path(__file__).with_name('app.html').read_text()}]}
    else: result={}
    print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
