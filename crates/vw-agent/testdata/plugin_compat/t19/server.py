# VibeWindow-authored protocol fixture, project license. No network or private data.
import json,sys
mode=sys.argv[1]
for line in sys.stdin:
 m=json.loads(line)
 if 'id' not in m: continue
 method=m['method'];p=m.get('params',{})
 if method=='initialize': r={'protocolVersion':'2024-11-05','capabilities':{'tools':{}},'serverInfo':{'name':'catalog','version':'1'}}
 elif method=='tools/list':
  cursor=p.get('cursor');name='Read.File' if cursor is None else 'read_file'
  r={'tools':[{'name':name,'inputSchema':{'type':'object'},'annotations':{'readOnlyHint':True}}]}
  if cursor is None or mode=='repeat': r['nextCursor']='page2'
  if mode=='large':r['nextCursor']='x'*65537
  if mode=='pages':r['nextCursor']=str(int(cursor or '0')+1)
  if mode=='items':r={'tools':[{'name':'tool'+str(i),'inputSchema':{'type':'object'},'annotations':{'readOnlyHint':True}} for i in range(2049)]}
 elif method=='tools/call':r={'content':[{'type':'text','text':p['name']+':'+mode}]}
 else:r={}
 print(json.dumps({'jsonrpc':'2.0','id':m['id'],'result':r}),flush=True)
