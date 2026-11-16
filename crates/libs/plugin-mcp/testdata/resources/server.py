# VibeWindow-authored fixture. MIT, matching the repository license.
import json,pathlib,sys,time
origin=sys.argv[1]
log=pathlib.Path(sys.argv[2])
mode=sys.argv[3] if len(sys.argv)>3 else 'normal'
for line in sys.stdin:
    req=json.loads(line)
    if 'id' not in req: continue
    method=req['method'];params=req.get('params',{})
    with log.open('a') as f:f.write(json.dumps({'method':method,'params':params})+'\n')
    if method=='initialize':
        result={'protocolVersion':'2024-11-05','capabilities':{'resources':{'subscribe':True},'prompts':{}},'serverInfo':{'name':origin,'version':'1'}}
        if mode=='no-cap':result['capabilities']={}
    elif method in ('resources/list','resources/templates/list','prompts/list'):
        cursor=params.get('cursor');key={'resources/list':'resources','resources/templates/list':'resourceTemplates','prompts/list':'prompts'}[method]
        item={'name':('second' if cursor else 'first'),'_meta':{'source':origin,'server':'forged'}}
        if key=='resources':item.update(uri='fixture:///'+item['name'],mimeType='text/plain',size=2**40)
        elif key=='resourceTemplates':item.update(uriTemplate='fixture:///{name}',mimeType='text/plain')
        else:item.update(arguments=[{'name':'question','required':True,'description':'Input'}])
        if mode=='items':result={key:[item]*2049}
        else:
            result={key:[item]}
            if not cursor or mode in ('repeat','pages','large-cursor'):result['nextCursor']='X'*65537 if mode=='large-cursor' else (str(int(cursor or 0)+1) if mode=='pages' else 'next')
    elif method=='resources/read':
        uri=params['uri']
        if uri=='fixture:///slow':time.sleep(2)
        result={'contents':[{'uri':uri,'mimeType':'text/plain','text':origin+':'+uri,'_meta':{'marker':'host-only'}},{'uri':'fixture:///blob','mimeType':'application/octet-stream','blob':'AAEC','_meta':{'source':origin}}],'_meta':{'origin':origin}}
        if uri=='fixture:///invalid':result['contents'][0]['text']=10
    elif method=='prompts/get':
        args=params.get('arguments',{})
        if 'question' not in args:
            print(json.dumps({'jsonrpc':'2.0','id':req['id'],'error':{'code':-32602,'message':'host-only-error-detail','data':{'private':'host-only-error-detail'}}}),flush=True);continue
        result={'description':origin,'messages':[{'role':'user','content':{'type':'text','text':args.get('question','missing'),'_meta':{'marker':'host-only'}}},{'role':'assistant','content':{'type':'resource','resource':{'uri':'fixture:///embedded','mimeType':'text/plain','text':origin,'_meta':{'source':origin}}}}],'_meta':{'server':'forged'}}
        if params['name']=='invalid':result['messages'][0]['role']='system'
    else:
        print(json.dumps({'jsonrpc':'2.0','id':req['id'],'error':{'code':-32601,'message':'unsupported'}}),flush=True);continue
    print(json.dumps({'jsonrpc':'2.0','id':req['id'],'result':result}),flush=True)
